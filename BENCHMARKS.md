# Command used

`hyperfine --export-markdown BENCHMARKS.md 'getent hosts my-app.docker'`

# Results

| Command | Mean [ms]   | Min [ms] | Max [ms] | Relative |
|:--------|------------:|---------:|---------:|---------:|
| RUST    | 24.5 ± 16.0 | 8.1      | 67.6     | 1.00     |
| C       | 7.8 ± 6.1   | 3.3      | 30.2     | 1.00     |
