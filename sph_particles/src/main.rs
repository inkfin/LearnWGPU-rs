use example_fish::run;

fn main() {
    pollster::block_on(run());
}
