// Example demonstrating the function call resolution bug and fix

fn main() {
    helper1();
    helper2();
    println!("Main function complete");
}

fn helper1() {
    println!("Helper 1 called");
}

fn helper2() {
    println!("Helper 2 called");
}
