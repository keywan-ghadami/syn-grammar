fn main() {
    let ts = "fn foo( const std::".parse::<proc_macro2::TokenStream>();
    println!("{:?}", ts);
}
