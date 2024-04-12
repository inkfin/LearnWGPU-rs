use bitonic_sort::run;

fn main() {
    pollster::block_on(run());
}
