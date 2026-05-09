pub fn combinations(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    if k == 0 || k == n { return 1; }
    let k = k.min(n - k);
    let mut res = 1;
    for i in 1..=k { res = res * (n - i + 1) / i; }
    res
}

pub fn format_with_separators(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push(','); }
        result.push(c);
    }
    result
}

pub fn format_currency(n: f64) -> String {
    let is_neg = n < 0.0;
    let s = format!("{:.0}", n.abs());
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 { result.push(','); }
        result.push(c);
    }
    if is_neg { format!("-${}", result) } else { format!("${}", result) }
}
