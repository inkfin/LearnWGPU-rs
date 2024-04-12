
# Parallel Bitonic Sort Algorithms

Implement parallel bitonic sort algorithms with Rust and Wgpu.

- size = $2^{8}$
  - CPU faster than GPU

```log
[2024-04-12T09:56:02Z INFO  bitonic_sort] Bitonic sort successful!
[2024-04-12T09:56:02Z INFO  bitonic_sort] Initialization takes: 0.4548751s
[2024-04-12T09:56:02Z INFO  bitonic_sort] Wgpu computation takes: 0.0055504s
[2024-04-12T09:56:02Z INFO  bitonic_sort] Data transfer takes: 0.0001467s
[2024-04-12T09:56:02Z INFO  bitonic_sort] CPU sorting takes: 0.000011s
```

- size = $2^{18}$
  - Similar performance

```log
[2024-04-12T09:57:18Z INFO  bitonic_sort] Bitonic sort successful!
[2024-04-12T09:57:18Z INFO  bitonic_sort] Initialization takes: 0.4637676s
[2024-04-12T09:57:18Z INFO  bitonic_sort] Wgpu computation takes: 0.0214897s
[2024-04-12T09:57:18Z INFO  bitonic_sort] Data transfer takes: 0.0005794s
[2024-04-12T09:57:18Z INFO  bitonic_sort] CPU sorting takes: 0.0217035s
```

- size = $2^{25}$
  - GPU faster than CPU

```log
[2024-04-12T09:53:07Z INFO  bitonic_sort] Bitonic sort successful!
[2024-04-12T09:53:07Z INFO  bitonic_sort] Initialization takes: 0.9346608s
[2024-04-12T09:53:07Z INFO  bitonic_sort] Wgpu computation takes: 0.1369619s
[2024-04-12T09:53:07Z INFO  bitonic_sort] Data transfer takes: 0.9909169s
[2024-04-12T09:53:07Z INFO  bitonic_sort] CPU sorting takes: 4.1491945s
```

