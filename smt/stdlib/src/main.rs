// Test Rust BigInt division behavior (your current implementation)
use num_bigint::BigInt;
use num_traits::Euclid;

fn main() {
    println!("=== Rust BigInt / operator (truncates toward zero) ===\n");

    let test_cases = vec![(-7, 3), (7, 3), (-7, -3), (7, -3)];

    for (a, b) in test_cases {
        let dividend = BigInt::from(a);
        let divisor = BigInt::from(b);

        let result = &dividend.div_euclid(&divisor);
        println!("quotient: {} / {} = {}", a, b, result);

        let result = &dividend.rem_euclid(&divisor);
        println!("reminder: {} % {} = {}", a, b, result);

        // (simplify (div_trunc (- 7) 3))
        // (simplify (div_trunc 7 3))
        // (simplify (div_trunc (- 7) (- 3)))
        // (simplify (div_trunc 7 (- 3)))

        // (simplify (rem_trunc (- 7) 3))
        // (simplify (rem_trunc 7 3))
        // (simplify (rem_trunc (- 7) (- 3)))
        // (simplify (rem_trunc 7 (- 3)))
    }
}
