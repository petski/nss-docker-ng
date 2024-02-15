# Command used

`hyperfine --export-markdown BENCHMARKS.md 'getent hosts my-app.docker'`

# Results

| Command | Mean [ms]   | Min [ms] | Max [ms] | Relative |
|:--------|------------:|---------:|---------:|---------:|
| RUST    | 32.9 ± 15.8 | 11.3     | 58.6     | 1.00     |
| C       | 7.8 ± 6.1   | 3.3      | 30.2     | 1.00     |
