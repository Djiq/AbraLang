use crate::*;
use paste::paste;

macro_rules! abra_compile_test {
    ($s1:ident $n1:ident; $($s2:ident $n2:ident);*) => {
        abra_compile_test!{$s1 $n1}
        abra_compile_test!{$($s2 $n2);*}
    };
    (ok $name:ident ) => {
        paste!{
            #[test]
            fn [< test_ $name >]() {
                let ret = $name();
                if ret.1.is_err(){
                    panic!("Error!");
                }
                let ret_code = ret.1.unwrap();
                if ret_code != 0 {
                    println!("AbraASM:\n{}",ret.0.unwrap().string_representation());
                    panic!("Expected 0 found {}",ret_code)
                }

            }
        
            abra_compile_test!{$name}
        }
    };
    (fail $name:ident ) => {
        paste!{
            #[test]
            fn [< test_ $name >]() {
                let ret = $name();
                if ret.1.is_err(){
                    panic!("Error!");
                }
                let ret_code = ret.1.unwrap();
                if ret_code == 0 {
                    println!("AbraASM:\n{}",ret.0.unwrap().string_representation());
                    panic!("Expected non-zero found {}",ret_code)
                }

            }
        
            abra_compile_test!{$name}
        }
    };
    (panic $name:ident ) => {
        paste!{
            #[test]
            #[should_panic]
            fn [< test_ $name >]() {
                let ret = $name();
                if ret.1.is_err(){
                    panic!("Error!");
                }
            }
        
            abra_compile_test!{$name}
        }
    };
    ($name:ident) => {
    fn $name() -> (Option<Code>,Result<u64>){
        let x = match tokenize(read_to_string(format!("tests/{}.abra",stringify!($name))).unwrap()){
            Ok(z) => z,
            Err(err) => {
                println!("Tokenization Error!\n{:?}",err);
                return (None,Err(err))
            }
        };
        let mut parser: Parser = Parser::new(x);
        let parsed = match parser.parse() {
            Ok(z) => z,
            Err(err) => {
                for cause in err.chain() {
                    println!("{:?}",cause);
                }
                println!("Parsing Error!\n{:?}",err);
                return (None,Err(err))
            }
        };
        let mut compiler : Compiler = Compiler::new();
        compiler.compile_from_ast(parsed);
        let code: Code = compiler.into();
        return (Some(code.clone()),Ok(ByteCodeMachine::new(code,false).run() as u64));
        }
    };
    
    
}   
abra_compile_test!{
    ok ok;
    fail fail;
    ok if_true;
    ok if_false;
    ok if_branch1;
    ok if_branch2;
    ok eq;
    ok neq;
    ok for_loop;
    ok var_decl;
    fail var_drop;
    ok fn_call
}

