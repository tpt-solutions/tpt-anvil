fn main() {
    let values = vec![1, 2, 3, 4, 5];
    let sum = values.iter().fold(0, |acc, x| acc + x);
    println!("Sum: {sum}");
}

fn second_half(values: &[i32]) -> &[i32] {
    // BUG (under test): `len() / 2` is wrong for odd lengths. For
    // `[1,2,3,4,5]` this yields `&[]` instead of `&[3,4,5]`, and for
    // `[1,2,3,4]` it yields `&[3,4]` by accident.
    let mid = values.len() / 2;
    &values[mid..]
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
