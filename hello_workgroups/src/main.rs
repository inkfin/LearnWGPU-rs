use hello_workgroups::run;

fn main() {
    pollster::block_on(run());
}
