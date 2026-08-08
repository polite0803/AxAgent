// 测试多行字符串拼接
fn main() {
    let s = "hello "
        "world";
    println!("{}", s);
    
    // 测试在函数调用中
    test_fn(
        "第一行\n"
        "第二行\n"
        "第三行"
    );
}

fn test_fn(s: &str) {
    println!("{}", s);
}