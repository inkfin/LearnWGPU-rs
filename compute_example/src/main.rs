use compute_example::run;

fn main() {
    pollster::block_on(run());
}
