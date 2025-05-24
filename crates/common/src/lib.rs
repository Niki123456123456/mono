pub mod app;
pub mod remove_where;
pub mod http;

pub use remove_where::RemoveWhere;

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
pub fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}


fn make_static_mut<T>(value: T) -> &'static std::sync::Mutex<T> {
    Box::leak(Box::new(std::sync::Mutex::new(value)))
}

fn make_static<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

struct Callback<T> {
    inner: std::sync::Arc<std::sync::Mutex<Option<Box<dyn FnOnce(T) + Send + Sync>>>>,
}

impl<T: 'static> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static, X> From<X> for Callback<T>
where
    X: FnOnce(T) + Send + Sync + 'static,
{
    fn from(value: X) -> Self {
        Callback::new(value)
    }
}

impl<T: 'static> Callback<T> {
    pub fn new(f: impl FnOnce(T) + Send + Sync + 'static) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(Some(Box::new(f)))),
        }
    }

    pub fn as_fn(&self) -> impl Fn(T) + Sync + Send + 'static {
        let inner = self.inner.clone();
        move |t| {
            if let Some(f) = inner.lock().unwrap().take() {
                f(t);
            }
        }
    }
}

pub fn thousands_sep(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count != 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    result.chars().rev().collect()
}
