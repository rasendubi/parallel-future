use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_std::task;

use futures_concurrency::prelude::*;
use parallel_future::prelude::*;

#[test]
fn spawn() {
    async_std::task::block_on(async {
        let res = async { "nori is a horse" }.par().await;
        assert_eq!(res, "nori is a horse");
    })
}

#[test]
fn with_join() {
    async_std::task::block_on(async {
        // A parallel execution of two futures
        let a = async { 1 }.par();
        let b = async { 2 }.par();

        let res: [i32; 2] = (a, b).join().await.into();
        assert_eq!(res.iter().sum::<i32>(), 3);
    })
}

#[test]
fn is_lazy() {
    async_std::task::block_on(async {
        let polled = Arc::new(Mutex::new(false));
        let polled_2 = polled.clone();
        let _res = async move {
            *polled_2.lock().unwrap() = true;
        }
        .par();

        task::sleep(Duration::from_millis(500)).await;
        assert_eq!(*polled.lock().unwrap(), false);
    })
}
