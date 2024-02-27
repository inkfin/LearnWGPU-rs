use hello_compute::run;

fn main() {
    pollster::block_on(run());
}
