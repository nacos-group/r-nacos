
use std::{io::{Cursor, SeekFrom}, sync::Arc, path::Path};

use actix::prelude::*;
use bean_factory::{bean, Inject};
use binrw::{BinWriterExt, BinReaderExt};
use quick_protobuf::{Writer, BytesReader};
use tokio::{fs::OpenOptions, io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}};

use crate::{common::protobuf_utils::{read_varint64, inner_sizeof_varint, read_varint64_offset, MessageBufReader, write_varint64,FileMessageReader}};
use super::log::{LogRecord, LogRange, RaftIndex};

use super::{model::{LogIndexHeaderDo, LogRecordDto},raftindex::{RaftIndexManager, RaftIndexResponse, RaftIndexRequest}};

const LOG_DATA_BUF_SIZE:u64= 1024*1024;

#[derive(Debug,Default,Clone)]
struct InnerIdxDto {
    log_index: u64,
    file_index:u64,
}

pub enum LogWriteMark{
    Success,
    SuccessToEnd,
    Failure,
    Error,
}

pub enum LogWriteResult{
    Success,
    SuccessToEnd(u64),
    Failure(u64,LogRecordDto),
    FailureBatch(u64,Box<Vec<LogRecordDto>>,usize),
    Error,
}

pub struct LogInnerManager {
    data_file:tokio::fs::File,
    index_file:tokio::fs::File,
    header: LogIndexHeaderDo,
    indexs: Vec<InnerIdxDto>,
    start_index: u64,
    index_cursor:u64,
    file_len: u64,
    data_cursor:u64,
    msg_count: u64,
    current_index_count:u16,
    need_seek_at_write: bool,
}

impl ToString for LogInnerManager {
    fn to_string(&self) -> String {
        format!("header:{:?},indexs count:{},msg_count:{},index_cursor:{},data_cursor:{},file_len:{}"
            ,&self.header,self.indexs.len(),self.msg_count,self.index_cursor,self.data_cursor,self.file_len)
    }
}

impl LogInnerManager {

    pub async fn init(log_path:String,start_index:u64) -> anyhow::Result<LogInnerManager> {
        let index_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&log_path).await?;
        let mut data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&log_path).await?;
        //let index_meta = index_file.metadata().await?;
        let data_meta = data_file.metadata().await?;
        let first_index = InnerIdxDto {
            log_index: start_index,
            file_index: 4096,
        };
        let (header,indexs,index_cursor,file_len) = if data_meta.len() == 0 {
            //init 
            let header = LogIndexHeaderDo::default();
            let data_buf = vec![0u8;256];
            let mut stream = Cursor::new(data_buf);
            stream.write_be(&header)?;
            stream.set_position(0);
            data_file.write(stream.get_mut()).await?;
            data_file.set_len(LOG_DATA_BUF_SIZE).await?;
            data_file.flush().await?;
            (header,vec![first_index],24,LOG_DATA_BUF_SIZE)
        }
        else{
            //load
            let mut data_buf = vec![0u8;4096+10];
            let _len = data_file.read(&mut data_buf).await?;
            let mut stream = Cursor::new(&data_buf);
            let header: LogIndexHeaderDo =  stream.read_be()?;
            let (indexs,index_cursor) = Self::read_indexs(&data_buf[24..],first_index,header.index_interval as u64)?;
            let file_len = data_meta.len();
            (header,indexs,index_cursor+24,file_len)
        };
        let (data_cursor,msg_count) = Self::move_to_end(&mut data_file,indexs.last().unwrap(),start_index).await?;
        data_file.seek(SeekFrom::Start(data_cursor)).await?;
        println!("data_cursor:{},{},{}",data_cursor,data_meta.len(),data_cursor==data_meta.len());
        let current_index_count = (msg_count%(header.index_interval as u64)) as u16;
        Ok(LogInnerManager {
            data_file,
            index_file,
            header,
            indexs,
            start_index,
            index_cursor,
            file_len,
            data_cursor,
            msg_count,
            current_index_count,
            need_seek_at_write: false,
        })
    }

    fn read_indexs(index_buf:&[u8],first_index:InnerIdxDto,index_interval:u64) -> anyhow::Result<(Vec<InnerIdxDto>,u64)> {
        let mut indexs = vec![];
        let last_end = index_buf.len()-10;
        let mut offset = 0;
        let mut last_log_index = first_index.log_index;
        let mut last_file_index = first_index.file_index;
        indexs.push(first_index);
        let mut next_index = read_varint64_offset(index_buf,offset)?;
        while next_index > 0 {
            //println!("next_index:{},{}",&next_index,&offset);
            last_log_index += index_interval;
            last_file_index += next_index;
            let index_obj = InnerIdxDto {
                log_index: last_log_index,
                file_index: last_file_index,
            };
            indexs.push(index_obj);
            offset += inner_sizeof_varint(next_index);
            if offset > last_end {
                break;
            }
            next_index = read_varint64_offset(index_buf, offset).unwrap_or(0);
        }
        //println!("next_index:{},{},{}",&next_index,&offset,&indexs.last().unwrap().file_index);
        Ok((indexs,offset as u64))
    }

    async fn move_to_end(file:&mut tokio::fs::File,last_index:&InnerIdxDto,start_index:u64) -> anyhow::Result<(u64,u64)> {
        Self::move_to_index_by_count(file,last_index,start_index,0xffff).await
    }

    async fn move_to_index_by_count(file:&mut tokio::fs::File,last_index:&InnerIdxDto,start_index:u64,count:u64) -> anyhow::Result<(u64,u64)> {
        let mut data_cursor = last_index.file_index;
        let msg_count = last_index.log_index - start_index;
        let mut buffer = vec![0u8;1024];
        let mut reader = MessageBufReader::new();
        file.seek(SeekFrom::Start(data_cursor)).await?;
        let mut c=0;
        println!("move_to_index_by_count {:?},{},{}",last_index,&start_index,&count);
        loop {
            let read_len = file.read(&mut buffer).await?;
            if read_len == 0 {
                return Ok((data_cursor,msg_count));
            }
            reader.append_next_buf(&buffer[..read_len]);
            while let Some(v) = reader.next_message_vec() {
                c +=1;
                data_cursor += v.len() as u64;
                if c== count {
                    return Ok((data_cursor,msg_count + c))
                }
            }
            if reader.is_empty() {
                break;
            }
        }
        Ok((data_cursor,msg_count + c))
    }

    async fn move_to_end2(file:&mut tokio::fs::File,last_index:&InnerIdxDto,start_index:u64) -> anyhow::Result<(u64,u64)> {
        let mut file_reader = FileMessageReader::new(file.try_clone().await?,last_index.file_index);
        let (count,last_position) = file_reader.read_to_end().await?;
        let msg_count = last_index.log_index - start_index + count;
        Ok((last_position.get_end_position(),msg_count))
    }

    pub async fn write(&mut self,record: &LogRecordDto) -> anyhow::Result<LogWriteMark>{
        //let last_index = self.indexs.last().unwrap();
        //println!("write 001,{},{},{}",last_index.file_index,last_index.log_index,self.header.data_area_index);
        if self.index_cursor + 10>= self.header.data_area_index as u64  || self.data_cursor >= 2_000_000_000 {
            return Ok(LogWriteMark::Failure)
        }
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        
        writer.write_message(&record.to_record_do())?;
        if self.file_len<= self.data_cursor + buf.len() as u64 {
            self.file_len+= std::cmp::max(buf.len() as u64,LOG_DATA_BUF_SIZE);
            self.data_file.set_len(self.file_len).await?;
        }
        if self.need_seek_at_write {
            self.data_file.seek(SeekFrom::Start(self.data_cursor)).await?;
            self.need_seek_at_write = false;
        }
        self.data_file.write(&buf).await?;
        self.msg_count +=1;
        self.data_cursor += buf.len() as u64;
        self.current_index_count +=1;
        if self.current_index_count == self.header.index_interval {
            self.current_index_count=0;
            let index_data = write_varint64(self.data_cursor - self.indexs.last().unwrap().file_index);
            self.index_file.seek(SeekFrom::Start(self.index_cursor)).await?;
            self.index_file.write(&index_data).await?;
            self.index_cursor += index_data.len() as u64;
            self.indexs.push(InnerIdxDto { 
                log_index: self.msg_count + self.header.first_index, 
                file_index: self.data_cursor,
            });
        }
        if self.index_cursor + 10>= self.header.data_area_index as u64  || self.data_cursor >= 2_000_000_000 {
            return Ok(LogWriteMark::SuccessToEnd)
        }
        Ok(LogWriteMark::Success)
    }

    pub async fn strip_log_to(&mut self,end_index:u64) -> anyhow::Result<()> {
        let last_end_index = self.get_end_index();
        if end_index >= last_end_index {
            //log::warn!("the data is not enough to be strip");
            return Ok(())
        }
        let (index_dto,file_index_len,pop_index_count) = self.get_file_index_by_log_index(end_index)?;
        let empty_data = vec![0u8,1];
        if pop_index_count > 0 {
            for _i in 0..pop_index_count {
                self.indexs.pop();
            }
            self.index_cursor-=file_index_len;
            self.index_file.seek(SeekFrom::Start(self.index_cursor)).await?;
            self.index_file.write(&empty_data).await?;
            self.index_file.seek(SeekFrom::Start(self.index_cursor)).await?;
            self.index_file.flush().await?;
        }
        let current_index_count =end_index - index_dto.log_index;
        let (data_cursor,msg_count)=Self::move_to_index_by_count(&mut self.data_file,&index_dto,self.start_index,current_index_count).await?;
        self.data_cursor = data_cursor;
        self.msg_count = msg_count;
        self.current_index_count = current_index_count as u16;
        self.data_file.seek(SeekFrom::Start(self.data_cursor)).await?;
        self.data_file.write(&empty_data).await?;
        self.data_file.seek(SeekFrom::Start(self.data_cursor)).await?;
        self.data_file.flush().await?;
        Ok(())
    }

    fn get_file_index_by_log_index(&self,log_index:u64) -> anyhow::Result<(InnerIdxDto,u64,u64)>{
        let mut file_index_len = 0;
        let mut pop_index_count = 0;
        let mut last_index = self.indexs.last().unwrap();
        for item in self.indexs.iter().rev() {
            if item.log_index != last_index.log_index {
                file_index_len+= inner_sizeof_varint(last_index.log_index-item.log_index) as u64;
                last_index = item;
                pop_index_count +=1;
            }
            if item.log_index<=log_index {
                return Ok((item.clone(),file_index_len,pop_index_count))
            }
        }
        Err(anyhow::anyhow!("not found index,{}",&log_index))
    }

    pub async fn read_records(&mut self,start:u64,end:u64) -> anyhow::Result<Vec<LogRecordDto>> {
        let end_index = self.get_end_index();
        let mut rlist = vec![];
        let start = std::cmp::max(start,self.start_index);
        let end = std::cmp::min(end,end_index);
        if start >= end  {
            //error args
            return Ok(rlist)
        }
        let index = self.get_start_index(start);
        let msg_position = {
            let mut file_reader = FileMessageReader::new(self.data_file.try_clone().await?,index.file_index);
            file_reader.seek_start(index.file_index).await?;
            file_reader.read_index_position((start-index.log_index) as usize).await?
        };
        let mut c = end - start;
        let mut message_reader = MessageBufReader::new();
        self.data_file.seek(SeekFrom::Start(msg_position.position)).await?;
        while c>0 {
            while let Some(v) = message_reader.next_message_vec() {
                let mut reader = BytesReader::from_bytes(v);
                let item : LogRecord = reader.read_message(v)?;
                let dto = item.into();
                rlist.push(dto);
                c-=1;
                if c==0 {
                    break;
                }
            }
            let mut buf = vec![0u8;1024];
            let read_len= self.data_file.read(&mut buf).await?;
            if read_len==0 {
                break;
            }
            message_reader.append_next_buf(&buf[..read_len]);
        }
        self.need_seek_at_write=true;
        Ok(rlist)
    }

    pub fn get_end_index(&self) -> u64 {
        self.start_index + self.msg_count
    }

    pub async fn load_record(&mut self,start:u64,end:u64) -> anyhow::Result<()> {
        let end_index = self.start_index + self.msg_count;
        if start >= end || start>= end_index {
            //error args
            return Ok(())
        }
        if start < self.start_index || end > end_index {
            //error args
            return Ok(())
        }
        /* 
        let store_addr = if let Some(store_addr) = self.store_addr.as_ref() {
            store_addr
        }
        else{
            return Ok(())
        };
        */
        let index = self.get_start_index(start);
        let msg_position = {
            let mut file_reader = FileMessageReader::new(self.data_file.try_clone().await?,index.file_index);
            file_reader.seek_start(index.file_index).await?;
            file_reader.read_index_position((start-index.log_index) as usize).await?
        };
        let mut c = end - start;
        let mut message_reader = MessageBufReader::new();
        self.data_file.seek(SeekFrom::Start(msg_position.position)).await?;
        while c>0 {
            while let Some(v) = message_reader.next_message_vec() {
                let mut reader = BytesReader::from_bytes(v);
                let item : LogRecord = reader.read_message(v)?;
                //let dto = item.into();
                //rlist.push(dto);
                //TODO send to apply_store
                //store_addr.do_send(StoreLoadRequest::LogRecord(dto));
                c-=1;
                if c==0 {
                    break;
                }
            }
            let mut buf = vec![0u8;1024];
            let read_len= self.data_file.read(&mut buf).await?;
            if read_len==0 {
                break;
            }
            message_reader.append_next_buf(&buf[..read_len]);
        }
        self.need_seek_at_write = true;
        Ok(())
    }

    fn get_start_index(&self,start:u64) -> &InnerIdxDto {
        let i = match self.indexs.binary_search_by_key(&start, |e|e.log_index) {
            Ok(i) => i,
            Err(i) => if i==0 {
                0
            }
            else{
                i-1
            },
        };
        self.indexs.get(i).unwrap()
    }
}

/// 一个日志文件对应一个RaftLogActor
pub struct RaftLogActor {
    path: String,
    start_index: u64,
    //store_addr:Option<Addr<InnerStore>>,
    inner: Option<Box<LogInnerManager>>,
    swap_read_list: Option<Box<Vec<LogRecordDto>>>,
    swap_result: Option<LogWriteResult>,
}

impl RaftLogActor {
    pub fn new(path: String,start_index: u64) -> Self {
        Self {
            path,
            start_index,
            //store_addr,
            inner: None,
            swap_read_list: None,
            swap_result: None,
        }
    }

    fn init(&mut self,ctx:&mut Context<Self>) -> anyhow::Result<()> {
        self.init_inner_manager(ctx);
        Ok(())
    }

    fn init_inner_manager(&mut self,ctx:&mut Context<Self>) {
        let log_path = self.path.clone();
        let start_index = self.start_index.to_owned();
        //let store_addr = self.store_addr.clone();
        async move {
            LogInnerManager::init(log_path,start_index).await
        }
        .into_actor(self)
        .map(|r,act,ctx|{
            match r {
                Ok(v) => {
                    act.inner = Some(Box::new(v));
                },
                Err(e) => {
                    log::error!("RaftLogActor init_inner_manager error,{}",e);
                    ctx.stop();
                },
            }
        })
        .wait(ctx);
    }

    fn load_record(&mut self,ctx:&mut Context<Self>,start:u64,end:u64) {
        let mut inner = self.inner.take();
        async move {
            if let Some(inner) = &mut inner{
                inner.load_record(start, end).await.ok();
            }
            inner
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            act.inner=v;
        })
        .wait(ctx);
    }

    fn query_record(&mut self,ctx:&mut Context<Self>,start:u64,end:u64) -> Option<Box<Vec<LogRecordDto>>> {
        let mut inner = self.inner.take();
        async move {
            let mut list = None;
            if let Some(inner) = &mut inner{
                list = Some(Box::new(inner.read_records(start, end).await.unwrap_or_default()));
            }
            (inner,list)
        }
        .into_actor(self)
        .map(|(v,list),act,_ctx|{
            act.inner=v;
            act.swap_read_list = list;
        })
        .wait(ctx);
        self.swap_read_list.take()
    }

    fn write(&mut self,ctx:&mut Context<Self>,record:LogRecordDto) -> LogWriteResult {
        let mut inner = self.inner.take().unwrap();
        async move {
            let mark = inner.write(&record).await.unwrap_or(LogWriteMark::Error);
            let result = match mark {
                LogWriteMark::Success => LogWriteResult::Success,
                LogWriteMark::SuccessToEnd => LogWriteResult::SuccessToEnd(inner.get_end_index()),
                LogWriteMark::Failure => LogWriteResult::Failure(inner.get_end_index(),record),
                LogWriteMark::Error => LogWriteResult::Error,
            };
            (inner,result)
        }
        .into_actor(self)
        .map(|(v,result),act,_ctx|{
            act.inner=Some(v);
            act.swap_result = Some(result)
        })
        .wait(ctx);
        self.swap_result.take().unwrap_or(LogWriteResult::Error)
    }

    fn write_batch(&mut self,ctx:&mut Context<Self>,list:Box<Vec<LogRecordDto>>,record_start_index:usize) -> LogWriteResult {
        let mut inner = self.inner.take().unwrap();
        async move {
            let mut mark = LogWriteMark::Success;
            let mut last_index = record_start_index;
            for record in &list[record_start_index..] {
                mark = inner.write(record).await.unwrap_or(LogWriteMark::Failure);
                if let LogWriteMark::Failure = mark {
                    break;
                }
                last_index+=1;
            }
            let result = match mark {
                LogWriteMark::Success => LogWriteResult::Success,
                LogWriteMark::SuccessToEnd => {
                    if last_index+1 == list.len() {
                        LogWriteResult::SuccessToEnd(inner.get_end_index())
                    }
                    else{
                        LogWriteResult::FailureBatch(inner.get_end_index(),list,last_index+1)
                    }
                },
                LogWriteMark::Failure => LogWriteResult::FailureBatch(inner.get_end_index(),list, last_index),
                LogWriteMark::Error => LogWriteResult::Error,
            };
            (inner,result)
        }
        .into_actor(self)
        .map(|(v,r),act,_ctx|{
            act.inner=Some(v);
            act.swap_result = Some(r);
        })
        .wait(ctx);
        self.swap_result.take().unwrap_or(LogWriteResult::Error)
    }

    fn strip_log_to(&mut self,ctx:&mut Context<Self>,end_index:u64) {
        let mut inner = self.inner.take();
        async move {
            if let Some(inner) = &mut inner{
                inner.strip_log_to(end_index).await.ok();
            }
            inner
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            act.inner=v;
        })
        .wait(ctx);
    }
}

impl Actor for RaftLogActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("RaftLogActor startd,{}",&self.path);
        self.init(ctx).ok();
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<RaftLogResponse>")]
pub enum RaftLogRequest {
    Query{
        start:u64,
        end:u64,
    },
    Load{
        start:u64,
        end:u64,
    },
    Write(LogRecordDto),
    WriteBatch(Box<Vec<LogRecordDto>>,usize),
    StripLogToIndex(u64),
}

pub enum RaftLogResponse{
    None,
    QueryResult(Box<Vec<LogRecordDto>>),
    WriteResult(LogWriteResult),
}

impl Handler<RaftLogRequest> for RaftLogActor {
    type Result = anyhow::Result<RaftLogResponse>;

    fn handle(&mut self, msg: RaftLogRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftLogRequest::Load { start, end } => {
                self.load_record(ctx, start, end);
            },
            RaftLogRequest::Query { start, end } => {
                return Ok(RaftLogResponse::QueryResult(self.query_record(ctx, start, end).unwrap_or_default()));
            },
            RaftLogRequest::Write(record) => {
                let r = self.write(ctx, record);
                return Ok(RaftLogResponse::WriteResult(r))
            },
            RaftLogRequest::WriteBatch(list,list_index) => {
                let r = self.write_batch(ctx, list,list_index);
                return Ok(RaftLogResponse::WriteResult(r))
            },
            RaftLogRequest::StripLogToIndex(end_index) => {
                self.strip_log_to(ctx, end_index);
            }
        };
        Ok(RaftLogResponse::None)
    }
}


#[derive(Clone)]
pub struct LogRangeWrap{
    log_range:LogRange,
    log_actor: Option<Addr<RaftLogActor>>,
}

impl LogRangeWrap {
    pub fn new(log_range:LogRange) -> Self {
        Self {
            log_range,
            log_actor: None,
        }
    }

    pub fn get_log_range_end_index(&self) -> u64 {
        if self.log_range.is_close {
            self.log_range.start_index + self.log_range.record_count
        }
        else{
            u64::MAX
        }
    }
}

#[bean(inject)]
#[derive(Default)]
pub struct RaftLogManager {
    logs: Vec<LogRangeWrap>,
    current_log_actor:Option<Addr<RaftLogActor>>,
    base_path:Arc<String>,
    index_info: Option<RaftIndex>,
    //最后应用的日志，只用于启动后第一次加载
    last_applied_log: u64,
    //inner_store: Option<Addr<InnerStore>>,
    index_manager: Option<Addr<RaftIndexManager>>,
    swap_read_list: Option<Box<Vec<LogRecordDto>>>,
}

impl RaftLogManager {
    pub fn new(base_path:Arc<String>) -> Self {
        Self {
            base_path,
            current_log_actor: None,
            logs:Vec::new(),
            index_info: None,
            last_applied_log: 0,
            //inner_store: None,
            index_manager: None,
            swap_read_list: None,
        }
    } 
    fn init(&mut self, ctx: &mut Context<Self>) {
        self.load_index_info(ctx);
        self.build_log_actor(ctx);
    }

    fn load_index_info(&mut self, ctx: &mut Context<Self>) {
        //加载索引文件、构建raft日志
        let index_manager = self.index_manager.clone();
        async move {
            if let Some(index_manager) = &index_manager{
                index_manager.send(super::raftindex::RaftIndexRequest::LoadIndexInfo).await?
            }
            else{
                Err(anyhow::anyhow!("load_index_info is error, index_manager is node"))
            }
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            if let Ok(RaftIndexResponse::RaftIndexInfo { raft_index, last_applied_log }) = v {
                act.index_info = Some(raft_index);
                act.last_applied_log = last_applied_log;
            }
        })
        .wait(ctx);
    }


    fn build_log_actor(&mut self, ctx: &mut Context<Self>) {
        if let Some(raft_index) = &self.index_info {
            self.logs = raft_index.logs.iter().map(|l|LogRangeWrap::new(l.clone())).collect();
        }
        if self.logs.is_empty() {
            log::error!("raft index logs is empty!");
            return;
        }
        let start_index = if let Some(v)=self.index_info.as_ref().unwrap().snapshots.last() {
            v.end_index+1
        }
        else {
            0
        };
        for item in self.logs.iter_mut().rev() {
            let log_end_index = item.get_log_range_end_index();
            if log_end_index > start_index {
                let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",item.log_range.id)).to_string_lossy().into_owned();
                let log_actor_addr  = RaftLogActor::new(log_path,item.log_range.start_index).start();
                /*
                let load_reqeust = RaftLogRequest::Load { 
                    start: item.log_range.start_index, 
                    //开区间
                    end: std::cmp::min(log_end_index,self.last_applied_log+1),
                };
                log_actor_addr.do_send(load_reqeust);
                 */
                item.log_actor = Some(log_actor_addr);
            }
            else {
                break;
            }
        }
        let last_log_range = self.logs.last_mut().unwrap();
        self.current_log_actor = last_log_range.log_actor.clone();
    }

    fn load_record(&mut self,ctx:&mut Context<Self>,start:u64,end:u64) {
        for item in &mut self.logs{
            if start < item.get_log_range_end_index() || end >=item.log_range.start_index {
                let log_actor=if let Some(log_actor)=item.log_actor.as_ref() {
                    log_actor.clone()
                }
                else{
                    let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",item.log_range.id)).to_string_lossy().into_owned();
                    let log_actor_addr  = RaftLogActor::new(log_path,item.log_range.start_index).start();
                    item.log_actor = Some(log_actor_addr.clone());
                    log_actor_addr
                };
                log_actor.do_send(RaftLogRequest::Load { start, end });
            }
        }
    }

    fn query_record(&mut self,ctx:&mut Context<Self>,start:u64,end:u64) -> Option<Box<Vec<LogRecordDto>>> {
        //let mut rlist = vec![];
        let mut actor_logs = vec![];
        for item in &mut self.logs{
            if start < item.get_log_range_end_index() || end >=item.log_range.start_index {
                let log_actor=if let Some(log_actor)=item.log_actor.as_ref() {
                    log_actor.clone()
                }
                else{
                    let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",item.log_range.id)).to_string_lossy().into_owned();
                    let log_actor_addr  = RaftLogActor::new(log_path,item.log_range.start_index).start();
                    item.log_actor = Some(log_actor_addr.clone());
                    log_actor_addr
                };
                actor_logs.push(log_actor);
            }
        }
        self.query_record_by_log_actors(ctx,actor_logs,start,end)
    }

    fn query_record_by_log_actors(&mut self,ctx:&mut Context<Self>,log_actors:Vec<Addr<RaftLogActor>>,start:u64,end:u64) -> Option<Box<Vec<LogRecordDto>>> {
        async move {
            let mut rlist = vec![];
            for log_actor in log_actors {
                let request = RaftLogRequest::Query { start, end };
                if let Ok(Ok(RaftLogResponse::QueryResult(mut list)))  =log_actor.send(request).await {
                    rlist.append(&mut list);
                }
            }
            Some(Box::new(rlist))
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            act.swap_read_list = v;
        })
        .wait(ctx);
        self.swap_read_list.take()
    }

    fn switch_new_log(&mut self,ctx:&mut Context<Self>,next_index:u64) {
        let next_log_id = {
            if let Some(last_log) = self.logs.last_mut() {
                last_log.log_range.is_close=true;
                last_log.log_range.record_count = next_index - last_log.log_range.start_index;
                last_log.log_range.id+1
            }
            else{
                0
            }
        };
        let new_log_range = LogRange { id: next_log_id, start_index: next_index, record_count: 0, is_close: false, mark_remove: false };
        let mut save_logs:Vec<LogRange> = self.logs.iter().map(|e|e.log_range.clone()).collect();
        save_logs.push(new_log_range.clone());
        let index_request = RaftIndexRequest::SaveLogs(save_logs);
        self.index_manager.as_ref().unwrap().do_send(index_request);
        let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",new_log_range.id)).to_string_lossy().into_owned();
        let log_actor_addr  = RaftLogActor::new(log_path,new_log_range.start_index).start();
        self.logs.push(LogRangeWrap { log_range: new_log_range, log_actor: Some(log_actor_addr.clone())});
        self.current_log_actor = Some(log_actor_addr);
    }

    fn write(&mut self,ctx:&mut Context<Self>,record:LogRecordDto,can_rewrite: bool) {
        let log_actor = self.current_log_actor.clone().unwrap();
        async move {
            let r=log_actor.send(RaftLogRequest::Write(record)).await??;
            Ok((r,can_rewrite))
        }
        .into_actor(self)
        .map(|v:anyhow::Result<(RaftLogResponse,bool)>,act,ctx|{
            if let Ok((v,can_rewrite)) = v {
                match v {
                    RaftLogResponse::WriteResult(write_result) => {
                        match write_result {
                            LogWriteResult::SuccessToEnd(next_index) => {
                                act.switch_new_log(ctx, next_index);
                            },
                            LogWriteResult::Failure(next_index,record) => {
                                act.switch_new_log(ctx, next_index);
                                if can_rewrite {
                                    act.write(ctx, record,false);
                                }
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        })
        .wait(ctx);
    }

    fn write_batch(&mut self,ctx:&mut Context<Self>,records:Box<Vec<LogRecordDto>>,record_index:usize) {
        let log_actor = self.current_log_actor.clone().unwrap();
        async move {
            let r=log_actor.send(RaftLogRequest::WriteBatch(records,record_index)).await??;
            Ok(r)
        }
        .into_actor(self)
        .map(|v:anyhow::Result<RaftLogResponse>,act,ctx|{
            if let Ok(v) = v {
                match v {
                    RaftLogResponse::WriteResult(write_result) => {
                        match write_result {
                            LogWriteResult::SuccessToEnd(next_index) => {
                                act.switch_new_log(ctx, next_index);
                            },
                            LogWriteResult::FailureBatch(next_index,records, record_index) => {
                                act.switch_new_log(ctx, next_index);
                                act.write_batch(ctx, records, record_index);
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        })
        .wait(ctx);
    }

    fn strip_log_to_index(&mut self,ctx:&mut Context<Self>,end_index:u64) {
        let mut pop_count=0;
        for item in &self.logs{
            if end_index < item.get_log_range_end_index() {
                let log_actor=if let Some(log_actor)=item.log_actor.as_ref() {
                    log_actor.clone()
                }
                else{
                    let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",item.log_range.id)).to_string_lossy().into_owned();
                    let log_actor_addr  = RaftLogActor::new(log_path,item.log_range.start_index).start();
                    log_actor_addr
                };
                log_actor.do_send(RaftLogRequest::StripLogToIndex(end_index));
                let is_remove = end_index < item.log_range.start_index;
                if is_remove {
                    pop_count+=1;
                }
            }
            else{
                break;
            }
        }
        if pop_count > 0 {
            let log_count = self.logs.len()-pop_count;
            self.logs = self.logs[..log_count].to_vec();
            if let Some(last_log) = self.logs.last_mut() {
                let log_actor = if let Some(log_actor)=&last_log.log_actor {
                    log_actor.clone()
                }
                else {
                    let log_path = Path::new(self.base_path.as_ref()).join(format!("log_{}",last_log.log_range.id)).to_string_lossy().into_owned();
                    let log_actor_addr  = RaftLogActor::new(log_path,last_log.log_range.start_index).start();
                    last_log.log_actor = Some(log_actor_addr.clone());
                    log_actor_addr
                };
                self.current_log_actor = Some(log_actor);
            }
        }
    }

}

impl Actor for RaftLogManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("RaftLogManager started");
    }
}

impl Inject for RaftLogManager {
    type Context = Context<Self>;

    fn inject(&mut self, factory_data: bean_factory::FactoryData, _factory: bean_factory::BeanFactory, ctx: &mut Self::Context) {
        self.index_manager = factory_data.get_actor();
        //self.inner_store = factory_data.get_actor();
        self.init(ctx);
    }
}

impl Handler<RaftLogRequest> for RaftLogManager {
    type Result=anyhow::Result<RaftLogResponse>;

    fn handle(&mut self, msg: RaftLogRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftLogRequest::Load { start, end } => {
                self.load_record(ctx, start, end);
                Ok(RaftLogResponse::None)
            },
            RaftLogRequest::Query { start, end } => {
                Ok(RaftLogResponse::QueryResult(self.query_record(ctx,start,end).unwrap_or_default()))
            },
            RaftLogRequest::Write(record) => {
                self.write(ctx, record, true);
                Ok(RaftLogResponse::None)
            },
            RaftLogRequest::WriteBatch(records,records_index) => {
                self.write_batch(ctx,records,records_index);
                Ok(RaftLogResponse::None)
            },
            RaftLogRequest::StripLogToIndex(end_index) => {
                self.strip_log_to_index(ctx, end_index);
                Ok(RaftLogResponse::None)
            },
        }
    }
}