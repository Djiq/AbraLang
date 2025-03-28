fn main() {
    lalrpop::Configuration::new().always_use_colors().set_out_dir("./src/").set_in_dir(".").process().unwrap();
}