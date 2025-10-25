use std::sync::Arc;

fn share_data(data: Arc<i32>) -> Arc<i32> {
    Arc::clone(&data)
}
