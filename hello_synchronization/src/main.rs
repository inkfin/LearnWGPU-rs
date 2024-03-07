use hello_synchronization::run;

fn main() {
    pollster::block_on(run());
}
