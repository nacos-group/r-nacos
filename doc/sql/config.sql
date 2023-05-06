create table if not exists tb_config(
    id integer primary key autoincrement,
    data_id varchar(255),
    `group` varchar(255),
    tenant varchar(255),
    content text,
    content_md5 varchar(36),
    last_time long
);
create index if not exists tb_config_key_idx on tb_config(data_id,`group`,tenant);

create table if not exists tb_config_history(
    id integer primary key autoincrement,
    data_id varchar(255),
    `group` varchar(255),
    tenant varchar(255),
    content text,
    last_time long
);
create index if not exists tb_config_history_key_idx on tb_config_history(data_id,`group`,tenant);

