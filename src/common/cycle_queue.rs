
#[derive(Debug)]
pub struct CycleQueue<T> {
    capacity: usize,
    array: Vec<Option<T>>,
    head: usize,
    tail: usize,
}

impl <T:ToOwned<Owned = T>> CycleQueue<T> {
    pub fn new(count: usize) -> Self {
        let capacity = count +1;
        let mut array = Vec::with_capacity(capacity.to_owned());
        for _ in 0..capacity {
            array.push(None);
        }
        Self {
            capacity,
            array,
            head:0,
            tail:0
        }
    }

    fn uidx(&self,i:usize) -> usize {
        if i < self.capacity {
            i
        }
        else{
            i - self.capacity 
        }
    }

    pub fn is_empty (&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full (&self) -> bool {
        self.uidx(self.tail + 1) == self.head
    }

    pub fn pushback(&mut self,item:T) -> Option<T> {
        let cell = self.array.get_mut(self.tail).unwrap();
        *cell = Some(item);
        let is_full = self.is_full();
        self.tail = self.uidx(self.tail + 1);
        if is_full {
            let cell = self.array.get_mut(self.head).unwrap();
            let v = cell.as_ref().map(|e|e.to_owned());
            *cell = None;
            self.head = self.uidx(self.head+1);
            v
        }
        else{
            None
        }
    }

    pub fn push(&mut self,item:T) {
        let cell = self.array.get_mut(self.tail).unwrap();
        *cell = Some(item);
        let is_full = self.is_full();
        self.tail = self.uidx(self.tail + 1);
        if is_full {
            let cell = self.array.get_mut(self.head).unwrap();
            *cell = None;
            self.head = self.uidx(self.head+1);
        }
    }

    pub fn remove_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        }
        else {
            let cell = self.array.get_mut(self.head).unwrap();
            let v = cell.as_ref().map(|e|e.to_owned());
            *cell = None;
            self.head = self.uidx(self.head+1);
            v
        }
    }

    pub fn len(&self) -> usize {
        if self.tail < self.head {
            self.capacity + self.tail - self.head
        }
        else {
            self.tail - self.head
        }
    }

    pub fn seek(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.array.get(self.head).unwrap().as_ref()
    }

    pub fn get(&self,index:usize) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        let idx = self.uidx(self.head+index);
        self.array.get(idx).unwrap().as_ref()
    }

}

#[cfg(test)]
mod tests {
    use super::CycleQueue;

    #[test]
    fn iter() {
        let mut queue : CycleQueue<u32> = CycleQueue::new(50);
        for i in 0..151 {
            queue.push(i);
        }
        assert_eq!(queue.remove_front(),Some(101));
        for i in 0..queue.len() {
            let v = queue.get(i);
            println!("item: {:?}",&v);
        }
    }
}