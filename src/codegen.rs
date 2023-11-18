use std::sync::Arc;

use crate::ir::IR;

pub trait Codegen {
    type Output: Into<String>;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output;
}

enum Level {
    Var(String),              // variable name
    Key(String, Arc<String>), // target object property name, variable name
}

impl From<&Level> for String {
    fn from(value: &Level) -> Self {
        match value {
            Level::Var(name) => name.clone(),
            Level::Key(name, prop) => format!("{}.{}", name, prop),
        }
    }
}

pub struct JSCodegen {
    varstack: Vec<Level>,
    arg: String,
}

impl JSCodegen {
    pub fn new(arg: String) -> Self {
        Self {
            varstack: Vec::new(),
            arg,
        }
    }

    fn topvar(&self) -> String {
        self.varstack
            .last()
            .map(|v| v.clone().into())
            .unwrap_or(self.arg.clone())
    }
}

impl Codegen for JSCodegen {
    type Output = String;

    fn generate<I: Iterator<Item = IR>>(mut self, it: I) -> Self::Output {
        use Level::*;
        let fnbody: Vec<String> = it
            .map(|op| match op {
                IR::G2G(_, _) => format!("{} = parseInt(input);", self.topvar()),
                IR::PushArr => {
                    let arrname = format!("arr{}", self.varstack.len()).to_string();
                    self.varstack.push(Var(arrname));
                    "".to_string()
                }
                IR::PushObj(key) => {
                    let objname = format!("obj{}", self.varstack.len()).to_string();
                    self.varstack.push(Key(objname.clone(), key.clone()));
                    format!("{} = {{}}", objname).to_string()
                }
                IR::Abs(_) => todo!(),
                IR::Del(_) => todo!(),
                IR::Inv => todo!(),
                IR::Pop => match self.varstack.pop() {
                    Some(_) => todo!(),
                    None => todo!(),
                },
                IR::Extr(_) => todo!(),
            })
            .collect();
        format!(
            "function({}) {{ {} return {}; }}",
            self.arg,
            fnbody.join(" "),
            self.topvar(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Ground;

    #[test]
    fn test_js_parse_int() {
        let code = JSCodegen::new("input".to_string())
            .generate(vec![IR::G2G(Ground::String, Ground::Num)].into_iter());
        assert_eq!(
            code,
            "function(input) { input = parseInt(input); return output; }"
        )
    }
}
