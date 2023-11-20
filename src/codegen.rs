use std::sync::Arc;

use itertools::Itertools;

use crate::ir::IR;

pub trait Codegen {
    type Output: Into<String>;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output;
}

enum Level {
    Var(String),      // variable name
    Key(Arc<String>), // target object property name, variable name
}

impl From<&Level> for String {
    fn from(value: &Level) -> Self {
        match value {
            Level::Var(name) => name.clone(),
            Level::Key(prop) => prop.to_string(),
        }
    }
}

impl From<Level> for String {
    fn from(value: Level) -> Self {
        match value {
            Level::Var(name) => name,
            Level::Key(prop) => prop.to_string(),
        }
    }
}

impl Level {
    fn is_key(&self) -> bool {
        match self {
            Level::Var(_) => false,
            Level::Key(_) => true,
        }
    }
}

pub struct JSCodegen {
    varstack: Vec<Level>,
    arg: String,
    retvar: String,
}

impl JSCodegen {
    pub fn new(arg: String, retvar: String) -> Self {
        Self {
            varstack: Vec::new(),
            arg,
            retvar,
        }
    }

    fn topvar(&self) -> String {
        self.varstack
            .last()
            .map(|v| v.into())
            .unwrap_or(self.retvar.clone())
    }

    fn poptop(&mut self) -> String {
        self.varstack
            .pop()
            .map(|v| (&v).into())
            .unwrap_or(self.retvar.clone())
    }

    fn input_path(&self) -> String {
        if self.varstack.is_empty() {
            self.arg.clone()
        } else {
            self.varstack
                .iter()
                .filter(|&l| l.is_key())
                .map(String::from)
                .join(".")
        }
    }
}

impl Codegen for JSCodegen {
    type Output = String;

    fn generate<I: Iterator<Item = IR>>(mut self, it: I) -> Self::Output {
        use Level::*;
        let fnbody: Vec<String> = it
            .map(|op| match op {
                IR::G2G(_, _) => format!("{} = parseInt({});", self.topvar(), self.input_path()),
                IR::PushArr => {
                    let arrname = format!("arr{}", self.varstack.len()).to_string();
                    self.varstack.push(Var(arrname));
                    "".to_string()
                }
                IR::PushObj(key) => {
                    let objname = format!("obj{}", self.varstack.len()).to_string();
                    self.varstack.push(Key(key.clone()));
                    format!("{} = {{}};", objname).to_string()
                }
                IR::Abs(_) => todo!(),
                IR::Del(_) => todo!(),
                IR::Inv => todo!(),
                IR::Pop => {
                    let var = self.poptop();
                    format!("")
                }
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
        let code = JSCodegen::new("input".to_string(), "output".to_string())
            .generate(vec![IR::G2G(Ground::String, Ground::Num)].into_iter());
        assert_eq!(
            code,
            "function(input) { output = parseInt(input); return output; }"
        )
    }

    #[test]
    fn test_js_parse_int_in_obj() {
        let code = JSCodegen::new("input".to_string(), "output".to_string()).generate(
            vec![
                IR::PushObj(Arc::new("foo".to_string())),
                IR::G2G(Ground::String, Ground::Num),
                IR::Pop,
            ]
            .into_iter(),
        );
        assert_eq!(
            code,
            "function(input) { obj0 = {}; obj0.foo = parseInt(input.foo); return obj0 }"
        )
    }
}
