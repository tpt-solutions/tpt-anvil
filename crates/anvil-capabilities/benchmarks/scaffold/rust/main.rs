fn main() {
    let values = vec![1, 2, 3, 4, 5];
    let sum = values.iter().fold(0, |acc, x| acc + x);
    println!("Sum: {sum}");
}

fn is_even(n: i32) -> bool {
    n % 2 == 0
}

fn first_positive(numbers: &[i32]) -> Option<i32> {
    for &n in numbers {
        if n > 0 {
            return Some(n);
        }
    }
    None
}
