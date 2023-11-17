use crate::ir::IR;

pub trait Codegen {
    type Output: Into<String>;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output;
}

pub struct JSCodegen;
impl Codegen for JSCodegen {
    type Output = String;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output {
        let fnbody: Vec<&'static str> = it
            .map(|op| match op {
                IR::G2G(_, _) => "input = parseInt(input);",
                IR::PushArr => todo!(),
                IR::PushObj(_) => todo!(),
                IR::Abs(_) => todo!(),
                IR::Del(_) => todo!(),
                IR::Inv => todo!(),
                IR::Pop => todo!(),
                IR::Extr(_) => todo!(),
            })
            .collect();
        format!("function(input) {{ {} return input; }}", fnbody.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Ground;

    #[test]
    fn test_js_parse_int() {
        let code = JSCodegen {}.generate(vec![IR::G2G(Ground::String, Ground::Num)].into_iter());
        assert_eq!(
            "function(input) { input = parseInt(input); return input; }",
            code
        )
    }
}