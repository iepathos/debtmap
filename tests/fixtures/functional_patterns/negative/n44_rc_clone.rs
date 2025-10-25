use std::rc::Rc;

fn duplicate_rc(data: Rc<i32>) -> Rc<i32> {
    Rc::clone(&data)
}
