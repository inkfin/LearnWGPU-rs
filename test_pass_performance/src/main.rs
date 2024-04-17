use test_pass_performance::run;

fn main() {
    pollster::block_on(run());
}
