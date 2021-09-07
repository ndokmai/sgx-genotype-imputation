#![feature(test)]
#![feature(bench_black_box)]
use colored::*;
use std::arch::x86_64::_rdtsc;
use std::hint::black_box;
use tp_fixedpoint::TpLnFixed;

type F = TpLnFixed<20>;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let save = if args.len() == 2 {
        if args[1] == "--save-results" {
            true
        } else {
            false
        }
    } else {
        false
    };

    let n = 100;
    let n_fold = 1000000;
    let alpha = 0.05;
    leading_zeros(n*10, n_fold*10, alpha, save);
    leftshift_tests(n*100, n_fold, alpha, save);
    rightshift_tests(n*100, n_fold, alpha, save);
    if_else_tests(n, n_fold, alpha, save);
    f32_tests(n, n_fold, alpha, save);
    const_select_tests(n, n_fold, alpha, save);
    fixed_tests(n, n_fold, alpha, save);
}

fn save_results(baseline: Vec<f64>, test: Vec<f64>, title: &str) {
    use std::fs::File;
    use std::io::Write;
    std::fs::create_dir_all("results").unwrap();
    let f_baseline_name = format!("results/{}_baseline.txt", title);
    let mut f_baseline = File::create(f_baseline_name).unwrap();
    baseline
        .into_iter()
        .for_each(|v| writeln!(&mut f_baseline, "{:.5}", v).unwrap());
    let f_test_name = format!("results/{}_test.txt", title);
    let mut f_test = File::create(f_test_name).unwrap();
    test.into_iter()
        .for_each(|v| writeln!(&mut f_test, "{:.5}", v).unwrap());
}


fn if_else_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "if-else";
    let title = format!("===== {} rank-sum test =====", title);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    println!("-------------------");
    let x_str = format!("time(if {{ EXPENSIVE }})");
    let u_str = format!("time(else {{ CHEAP }})");
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);

    let f = |cond| {
        black_box({
            let mut z = 0.0;
            for _ in 0..n_fold {
                black_box(if black_box(cond) {
                    z = black_box(black_box(1.0) * black_box(1e-38f32));
                } else {
                    z = black_box(black_box(1.0) * black_box(1.0));
                })
            }
            z
        });
    };

    let f_baseline = || f(true);
    let f_test = || f(false);
    let (baseline, test) = time_measure_all(n, f_baseline, f_test);
    let (baseline, test) = htest(baseline, test, alpha, n_fold);
    if save {
        save_results(baseline, test, "if-else");
    }
    println!("");
    println!("");
}

macro_rules! op {
    ($o: tt, $n_fold: expr) => {
        |x, y| {
            black_box({
                let mut z = x;
                for _ in 0..$n_fold {
                    z = black_box(black_box(x) $o black_box(y))
                };
                z
            }
            )
        }
    }
}

fn f32_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "f32";
    let a = (1f32, "1.0");
    let b = (1f32, "1.0");
    let c = (1e-38f32, "VERY_SMALL");

    let f = op! {+, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "+");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "add").as_str());
    }

    let f = op! {-, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "-");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "sub").as_str());
    }

    let f = op! {*, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "*");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "mult").as_str());
    }

    let f = op! {/, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "/");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "div").as_str());
    }
}

fn const_select_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "fixed-select";
    let title = format!("===== {} rank-sum test =====", title);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    const_select_template(true, false, n, n_fold, alpha);
    const_select_template(false, true, n, n_fold, alpha);
    let (baseline, test) = const_select_template(false, false, n, n_fold, alpha);
    if save {
        save_results(baseline, test, "fixed-select");
    }
    println!("");
    println!("");
}

fn fixed_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "fixed-time-ln";
    let a = (F::ONE, "ln(0.0)");
    let b = (F::ONE, "ln(0.0)");
    let c = (F::NAN, "ln(VERY_LARGE)");

    let f = op! {+, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "+");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "add").as_str());
    }

    let f = op! {-, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "-");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "sub").as_str());
    }

    let f = op! {*, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "*");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "mult").as_str());
    }

    let f = op! {/, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "/");
    if save {
        save_results(baseline, test, format!("{}_{}", title, "div").as_str());
    }
}

fn const_select_template(
    first: bool,
    second: bool,
    n: usize,
    n_fold: usize,
    alpha: f64,
) -> (Vec<f64>, Vec<f64>) {
    use timing_shield::TpBool;
    println!("-------------------");
    let x_str = format!("time({}-{} {{ EXPENSIVE }})", true, true);
    let u_str = format!("time({}-{} {{ CHEAP }})", first, second);
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);

    let f = |cond0, cond1| {
        black_box({
            let mut z = F::ONE;
            for _ in 0..n_fold {
                z = black_box(F::select_from_4_f32(
                    black_box(cond0),
                    black_box(cond1),
                    black_box(black_box(1.0) * black_box(1e-38f32)),
                    black_box(black_box(1.0) * black_box(1.0)),
                    black_box(black_box(1.0) * black_box(1.0)),
                    black_box(black_box(1.0) * black_box(1.0)),
                ));
            }
            z
        });
    };

    let f_baseline = || f(TpBool::protect(true), TpBool::protect(true));
    let f_test = || f(TpBool::protect(first), TpBool::protect(second));
    let (baseline, test) = time_measure_all(n, f_baseline, f_test);
    htest(baseline, test, alpha, n_fold)
}

fn op_template<N: Copy>(
    n: usize,
    n_fold: usize,
    alpha: f64,
    a: (N, &str),
    b: (N, &str),
    c: (N, &str),
    f: impl Fn(N, N) -> N + Copy,
    title: &str,
    op: &str,
) -> (Vec<f64>, Vec<f64>) {
    let title = format!("===== {} `{}` rank-sum test =====", title, op);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    let x_str = format!("time({} {} {})", a.1, op, b.1);
    let u_str = format!("time({} {} {})", a.1, op, c.1);
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);
    let f_baseline = move || f(a.0, b.0);
    let f_test = move || f(a.0, c.0);
    let (baseline, test) = time_measure_all(n, f_baseline, f_test);
    let (baseline, test) = htest(baseline, test, alpha, n_fold);
    println!("");
    println!("");
    (baseline, test)
}

fn time_measure_all<N>(
    n_rounds: usize,
    f_baseline: impl Fn() -> N,
    f_test: impl Fn() -> N,
) -> (Vec<u64>, Vec<u64>) {
    let mut baseline = Vec::with_capacity(n_rounds);
    let mut test = Vec::with_capacity(n_rounds);
    black_box(for _ in 0..n_rounds {
        baseline.push(time_measure_single(&f_baseline));
        test.push(time_measure_single(&f_test));
    });
    (baseline, test)
}

fn time_measure_single<N>(f: impl Fn() -> N) -> u64 {
    unsafe {
        let now = _rdtsc();
        let _ = black_box(f());
        _rdtsc() - now
    }
}

fn htest(baseline: Vec<u64>, test: Vec<u64>, alpha: f64, n_fold: usize) -> (Vec<f64>, Vec<f64>) {
    let htest = rustats::hypothesis_testings::MannWhitneyU::new(baseline.iter(), test.iter());
    println!("p-value = {:.3}", htest.p_value().unwrap());
    if htest.test(alpha) {
        println!("{}", "Possible timing leakage.".red().bold());
    } else {
        println!("{}", "Timing leakage NOT detected.".green().bold());
    }
    let baseline = baseline
        .into_iter()
        .map(|v| v as f64 / n_fold as f64)
        .collect();
    let test = test.into_iter().map(|v| v as f64 / n_fold as f64).collect();
    (baseline, test)
}

fn leading_zeros(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "leading zeros";
    let title = format!("===== {} rank-sum test =====", title);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    println!("-------------------");
    let x_str = format!("leading_zeros(0)");
    let u_str = format!("leading_zeros(LARGE_NUMBER)");
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);
    let f = |x: u64| {
        black_box({
            let mut z = 0;
            for _ in 0..n_fold {
                z = black_box(x.leading_zeros());
            }
            z
        });
    };

    let f_baseline = || f(black_box(0));
    let f_test = || f(black_box(0));
    let (baseline, test) = time_measure_all(n, f_baseline, f_test);
    let (baseline, test) = htest(baseline, test, alpha, n_fold);
    if save {
        save_results(baseline, test, "leading zeros");
    }
}

fn leftshift_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "left-shift";
    let a = (123456789101112u64, "123456789101112");
    let b = (1, "1");
    let c = (50, "50");

    let f = op! {<<, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "<<");
    if save {
        save_results(baseline, test, "left-shift");
    }
}

fn rightshift_tests(n: usize, n_fold: usize, alpha: f64, save: bool) {
    let title = "right-shift";
    let a = (123456789101112u64, "123456789101112");
    let b = (50, "50");
    let c = (50, "50");

    let f = op! {>>, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, ">>");
    if save {
        save_results(baseline, test, "right-shift");
    }
}
