#![feature(test)]
use colored::*;
use smac::ln_fixed::TpLnFixed;
use statrs::distribution::{Normal, Univariate};
use statrs::statistics::Statistics;
use std::arch::x86_64::_rdtsc;
use std::hint::black_box;

type F = TpLnFixed<typenum::U20>;

fn main() {
    let n = 10000;
    let n_fold = 1000;
    let alpha = 0.05;
    if_else_tests(n, alpha, n_fold);
    f32_tests(n, alpha, n_fold);
    f64_tests(n, alpha, n_fold);
    const_select_tests(n, alpha, n_fold);
    fixed_tests(n, alpha, n_fold);
}

fn save_results(baseline: Vec<f64>, test: Vec<f64>, title: &str) {
    use std::fs::File;
    use std::io::Write;
    let f_baseline_name = format!("results/{}_baseline.txt", title);
    let mut f_baseline = File::create(f_baseline_name).unwrap();
    baseline
        .into_iter()
        .for_each(|v| writeln!(&mut f_baseline, "{}", v).unwrap());
    let f_test_name = format!("results/{}_test.txt", title);
    let mut f_test = File::create(f_test_name).unwrap();
    test.into_iter()
        .for_each(|v| writeln!(&mut f_test, "{}", v).unwrap());
}

fn if_else_tests(n: usize, alpha: f64, n_fold: usize) {
    let title = "if-else";
    let title = format!("===== {} z-test =====", title);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    if_else_template(n, n_fold, alpha);
    println!("");
    println!("");
}

macro_rules! op {
    ($o: tt, $n_fold: expr) => {
    |x, y| (0..$n_fold).fold(x, |_, _| black_box(black_box(x) $o black_box(y)))
    }
}

fn f32_tests(n: usize, alpha: f64, n_fold: usize) {
    let title = "f32";
    let a = (1f32, "1.0");
    let b = (1f32, "1.0");
    let c = (1e-38f32, "VERY_SMALL");

    let f = op! {+, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "+");

    let f = op! {-, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "-");

    let f = op! {*, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "*");
    save_results(baseline, test, format!("{}_{}", title, "mult").as_str());

    let f = op! {/, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "/");
}

fn f64_tests(n: usize, alpha: f64, n_fold: usize) {
    let title = "f64";
    let a = (1f64, "1.0");
    let b = (1f64, "1.0");
    let c = (1e-320f64, "VERY_SMALL");

    let f = op! {+, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "+");

    let f = op! {-, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "-");

    let f = op! {*, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "*");

    let f = op! {/, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "/");
}

fn const_select_tests(n: usize, alpha: f64, n_fold: usize) {
    let title = "fixed-select";
    let title = format!("===== {} z-test =====", title);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    const_select_template(true, false, n, n_fold, alpha);
    const_select_template(false, true, n, n_fold, alpha);
    let (baseline, test) = const_select_template(false, false, n, n_fold, alpha);
    save_results(baseline, test, "fixed-select");
    println!("");
    println!("");
}

fn fixed_tests(n: usize, alpha: f64, n_fold: usize) {
    let title = "fixed-time-ln";
    let a = (F::ONE, "ln(0.0)");
    let b = (F::ONE, "ln(0.0)");
    let c = (F::NAN, "ln(VERY_LARGE)");

    let f = op! {+, n_fold};
    let (baseline, test) = op_template(n, n_fold, alpha, a, b, c, f, title, "+");
    save_results(baseline, test, format!("{}_{}", title, "add").as_str());

    let f = op! {-, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "-");

    let f = op! {*, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "*");

    let f = op! {/, n_fold};
    op_template(n, n_fold, alpha, a, b, c, f, title, "/");
}

fn if_else_template(n: usize, n_fold: usize, alpha: f64) {
    println!("-------------------");
    let x_str = format!("time(if {{ EXPENSIVE }})");
    let u_str = format!("time(else {{ CHEAP }})");
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);

    let f = |cond| {
        (0..n_fold).fold(1.0, |_, _| {
            black_box(if black_box(cond) {
                black_box(1.0) * black_box(1e-38f32)
            } else {
                black_box(1.0) * black_box(1.0)
            })
        });
    };

    let f_baseline = || f(true);
    let f_test = || f(false);
    let (baseline, test) = time_measure_all(n, n_fold, f_baseline, f_test);
    let (baseline, test) = ztest(baseline, test, alpha, &x_str, &u_str);
    save_results(baseline, test, "if-else");
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
        (0..n_fold).fold(TpLnFixed::ONE, |_, _| {
            black_box(F::select_from_4_f32(
                black_box(cond0),
                black_box(cond1),
                black_box(1.0) * black_box(1e-38f32),
                black_box(1.0) * black_box(1.0),
                black_box(1.0) * black_box(1.0),
                black_box(1.0) * black_box(1.0),
            ))
        });
    };

    let f_baseline = || f(TpBool::protect(true), TpBool::protect(true));
    let f_test = || f(TpBool::protect(first), TpBool::protect(second));
    let (baseline, test) = time_measure_all(n, n_fold, f_baseline, f_test);
    ztest(baseline, test, alpha, &x_str, &u_str)
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
    let title = format!("===== {} `{}` z-test =====", title, op);
    println!("{}", title.bold());
    println!("N = {}; alpha = {}", n, alpha);
    let x_str = format!("time({} {} {})", a.1, op, b.1);
    let u_str = format!("time({} {} {})", a.1, op, c.1);
    println!("H_0: {} == {}", x_str, u_str);
    println!("H_1: {} != {}", x_str, u_str);
    let f_baseline = move || f(a.0, b.0);
    let f_test = move || f(a.0, c.0);
    let (baseline, test) = time_measure_all(n, n_fold, f_baseline, f_test);
    let (baseline, test) = ztest(baseline, test, alpha, &x_str, &u_str);
    println!("");
    println!("");
    (baseline, test)
}

fn time_measure_all<N>(
    n_rounds: usize,
    n_fold: usize,
    f_baseline: impl Fn() -> N,
    f_test: impl Fn() -> N,
) -> (Vec<f64>, Vec<f64>) {
    let mut baseline = Vec::with_capacity(n_rounds);
    let mut test = Vec::with_capacity(n_rounds);
    let mut choices = vec![false; n_rounds];
    let mut rng = rand::thread_rng();
    use rand::Fill;
    choices.try_fill(&mut rng).unwrap();
    // warm up
    for &c in &choices {
        if c {
            f_baseline();
            f_test();
        } else {
            f_test();
            f_baseline();
        }
    }

    for &c in &choices {
        if c {
            baseline.push(time_measure_single(&f_baseline) as f64);
            test.push(time_measure_single(&f_test) as f64);
        } else {
            test.push(time_measure_single(&f_test) as f64);
            baseline.push(time_measure_single(&f_baseline) as f64);
        }
    }
    let baseline = baseline.into_iter().map(|v| v / n_fold as f64).collect();
    let test = test.into_iter().map(|v| v / n_fold as f64).collect();
    (baseline, test)
}

fn time_measure_single<N>(f: impl Fn() -> N) -> u64 {
    unsafe {
        let now = _rdtsc();
        let _ = black_box(f());
        _rdtsc() - now
    }
}

fn ztest(
    baseline: Vec<f64>,
    test: Vec<f64>,
    alpha: f64,
    x_str: &str,
    u_str: &str,
) -> (Vec<f64>, Vec<f64>) {
    let baseline = denoise(baseline);
    let test = denoise(test);

    let x = baseline.as_slice().mean();
    let s = baseline.as_slice().population_std_dev();
    let u = test.as_slice().mean();
    let z = (u - x) / s;
    let d = Normal::new(0., 1.).unwrap();
    let p = d.cdf(-z.abs()) * 2.;

    let x_str = format!("{} = {:.4} cycles", x_str, x);
    let u_str = format!("{} = {:.4} cycles", u_str, u);
    println!("{}", x_str.bright_blue().bold());
    println!("{}", u_str.yellow().bold());
    println!("std-dev = {:.3}", s);
    println!("p-value = {:.3}", p);
    if p < alpha {
        println!("{}", "Timing leakage detected.".red().bold());
    } else {
        println!("{}", "Timing leakage NOT detected.".green().bold());
    }
    (baseline, test)
}

// remove anything beyond 5 std_dev from mean
fn denoise(v: Vec<f64>) -> Vec<f64> {
    let x = v.as_slice().mean();
    let s = v.as_slice().std_dev();
    let k = 5.;
    v.into_iter()
        .filter(|&a| a < x + s * k && a > x - s * k)
        .collect()
}
