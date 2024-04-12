use bitonic_sort;

#[test]
fn bitonic_sort_num_group_array() {
    const ARRAY_LENGTH: usize = 16;
    let mut result_init = vec![];
    let mut result_curr = vec![];

    let log_len = (ARRAY_LENGTH as f32).log2() as u32;
    for num_stage in 1..=log_len {
        let log_num_group_init = log_len - num_stage;
        result_init.push(log_num_group_init.to_owned());
        for num_step in 0..num_stage {
            let log_num_group = log_num_group_init + num_step;
            result_curr.push(log_num_group);
        }
    }

    assert_eq!(result_init, vec![3, 2, 1, 0]);
    assert_eq!(result_curr, vec![3, 2, 3, 1, 2, 3, 0, 1, 2, 3])
}
