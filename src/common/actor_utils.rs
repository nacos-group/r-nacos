use actix::prelude::*;

pub fn create_actor_at_thread<T>(actor: T) -> Addr<T>
where
    T: Actor<Context = Context<T>> + Send,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let rt = System::new();
        let addrs = rt.block_on(async { actor.start() });
        tx.send(addrs).unwrap();
        rt.run().unwrap();
    });
    rx.recv().unwrap()
}

pub fn create_actor_at_thread2<T1, T2>(a: T1, b: T2) -> (Addr<T1>, Addr<T2>)
where
    T1: Actor<Context = Context<T1>> + Send,
    T2: Actor<Context = Context<T2>> + Send,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let rt = System::new();
        let addrs = rt.block_on(async { (a.start(), b.start()) });
        tx.send(addrs).unwrap();
        rt.run().unwrap();
    });
    rx.recv().unwrap()
}

pub fn create_actor_at_thread3<T1, T2, T3>(a: T1, b: T2, c: T3) -> (Addr<T1>, Addr<T2>, Addr<T3>)
where
    T1: Actor<Context = Context<T1>> + Send,
    T2: Actor<Context = Context<T2>> + Send,
    T3: Actor<Context = Context<T3>> + Send,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let rt = System::new();
        let addrs = rt.block_on(async { (a.start(), b.start(), c.start()) });
        tx.send(addrs).unwrap();
        rt.run().unwrap();
    });
    rx.recv().unwrap()
}
