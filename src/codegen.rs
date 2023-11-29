use std::{
    array, clone,
    fmt::{format, Display},
    sync::Arc,
};

use serde_json::json;

use crate::{ir::IR, schema::Ground};

pub trait Codegen {
    type Output: Into<String>;

    fn generate<I: Iterator<Item = IR>>(self, it: I) -> Self::Output;
}

#[derive(Clone, Debug)]
enum Level {
    Var(String),         // variable name
    Key(Arc<String>),    // target object property name
    Arr(String, String), // array name, index name
}

impl Level {
    fn key(k: &str) -> Self {
        Self::Key(Arc::new(k.to_string()))
    }

    fn var(v: &str) -> Self {
        Self::Var(v.to_string())
    }

    fn arr(a: &str, i: &str) -> Self {
        Self::Arr(a.to_string(), i.to_string())
    }
}

impl From<&Level> for String {
    fn from(value: &Level) -> Self {
        match value {
            Level::Var(name) => name.clone(),
            Level::Key(prop) => prop.to_string(),
            Level::Arr(name, _) => name.clone(),
        }
    }
}

impl From<Level> for String {
    fn from(value: Level) -> Self {
        match value {
            Level::Var(name) => name,
            Level::Key(prop) => prop.to_string(),
            Level::Arr(name, _) => name,
        }
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

pub struct JSCodegen {
    varstack: Vec<Level>,
    arg: Level,
    retvar: Level,
    uniq: usize,
}

impl JSCodegen {
    pub fn new(arg: &str, retvar: &str) -> Self {
        Self {
            varstack: Vec::new(),
            arg: Level::Var(arg.to_string()),
            retvar: Level::Var(retvar.to_string()),
            uniq: 0,
        }
    }

    fn peektop(&self) -> Level {
        self.varstack.last().unwrap_or(&self.retvar).clone()
    }

    fn poptop(&mut self) -> Level {
        self.varstack.pop().unwrap_or(self.retvar.clone())
    }

    fn output_path(&self) -> String {
        let top = self.peektop();
        match top {
            Level::Key(k) => {
                let v = self.varstack.iter().nth_back(1).unwrap_or(&self.retvar);
                if let Level::Var(v) = v {
                    format!("{}.{}", v, k)
                } else {
                    // TODO: Encode this invariant in type system
                    panic!("Top of varstack was key, but underneath was not a var")
                }
            }
            Level::Arr(arr, idx) => format!("{}[{}]", arr, idx),
            Level::Var(v) => v,
        }
    }

    fn input_path(&self) -> String {
        let mut buf = self.arg.to_string();
        if self.varstack.is_empty() {
            buf
        } else {
            for lvl in self.varstack.iter() {
                match lvl {
                    Level::Var(_) => (),
                    Level::Key(k) => {
                        buf.push('.');
                        buf.push_str(k)
                    }
                    Level::Arr(_, i) => {
                        buf.push('[');
                        buf.push_str(i);
                        buf.push(']');
                    }
                }
            }
            buf
        }
    }

    fn new_var(&mut self, prefix: &str) -> String {
        let varname = format!("{}{}", prefix, self.uniq);
        self.uniq += 1;
        varname.to_string()
    }

    fn generate_ground_to_ground(&self, from: Ground, to: Ground) -> Option<String> {
        Some(match (from, to) {
            (Ground::Num, Ground::Bool) => {
                format!("{} = !({} === 0);", self.output_path(), self.input_path())
            }
            (Ground::Bool, Ground::Num) => {
                format!("{} = {} ? 0 : 1;", self.output_path(), self.input_path())
            }
            (Ground::String, Ground::Num) => {
                format!("{} = parseInt({});", self.output_path(), self.input_path())
            }
            (Ground::String, Ground::Bool) => {
                format!("{} = !!({});", self.output_path(), self.input_path())
            }
            (Ground::Null, Ground::Num) => {
                format!("{} = 0;", self.output_path())
            }
            (Ground::Null, Ground::Bool) => {
                format!("{} = false;", self.output_path())
            }
            (Ground::Null, Ground::String) => {
                format!("{} = \"null\"", self.output_path())
            }
            (_, Ground::String) => {
                format!("{} = {}.toString();", self.output_path(), self.input_path())
            }
            (_, Ground::Null) => {
                format!("{} = null", self.output_path())
            }
            (_, _) => return None,
        })
    }
}

impl Codegen for JSCodegen {
    type Output = String;

    fn generate<I: Iterator<Item = IR>>(mut self, it: I) -> Self::Output {
        use Level::*;
        let mut frags = Vec::new();
        for op in it {
            match op {
                IR::G2G(from, to) => {
                    if let Some(frag) = self.generate_ground_to_ground(from, to) {
                        frags.push(frag)
                    }
                }
                IR::PushArr => {
                    let arrname = self.new_var("arr");
                    let idx = self.new_var("idx");
                    frags.push(format!("let {} = [];", arrname));
                    frags.push(format!(
                        "for (let {} = 0; {} < {}.length; {}++) {{",
                        idx,
                        idx,
                        self.input_path(),
                        idx,
                    ));
                    self.varstack.push(Arr(arrname.clone(), idx.clone()));
                }
                IR::PopArr => {
                    let popvar = self.poptop();
                    if let Level::Arr(var, _) = popvar {
                        frags.push("}".to_string());
                        frags.push(format!("{} = {};", self.output_path(), var));
                    } else {
                        panic!("PopArr instruction executed but top of stack was not arr");
                    }
                }
                IR::PushKey(_) => todo!(),
                IR::PopKey => todo!(),
                IR::PushObj => {
                    todo!()
                }
                IR::Abs(_) => todo!(),
                IR::Del(_) => todo!(),
                IR::Inv => todo!(),
                IR::Extr(_) => todo!(),
                IR::PopObj => todo!(),
            }
        }
        format!(
            "function({}) {{ {} return {}; }}",
            self.arg,
            frags.join(" "),
            self.retvar,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Ground;

    #[test]
    fn test_input_path() {
        let mut cg = JSCodegen::new("input", "output");
        cg.varstack = vec![
            Level::key("quux"),
            Level::var("obj0"),
            Level::arr("foo", "i"),
            Level::key("bar"),
        ];
        assert_eq!(cg.input_path(), "input.quux[i].bar")
    }

    #[test]
    fn test_js_parse_int() {
        let code = JSCodegen::new("input", "output")
            .generate(vec![IR::G2G(Ground::String, Ground::Num)].into_iter());
        assert_eq!(
            code,
            "function(input) { output = parseInt(input); return output; }"
        )
    }

    //#[test]
    fn test_js_parse_int_in_obj() {
        let code = JSCodegen::new("input", "output").generate(
            vec![
                IR::PushObj,
                IR::PushKey(Arc::new("foo".to_string())),
                IR::G2G(Ground::String, Ground::Num),
                IR::PopKey,
                IR::PopObj,
            ]
            .into_iter(),
        );
        assert_eq!(
            code,
            "function(input) { obj0 = {}; obj0.foo = parseInt(input.foo); return obj0 }"
        )
    }

    #[test]
    fn test_push_arr() {
        let code = JSCodegen::new("input", "output").generate(
            vec![
                IR::PushArr,
                IR::G2G(Ground::String, Ground::Num),
                IR::PopArr,
            ]
            .into_iter(),
        );
        assert_eq!(code, "function(input) { let arr0 = []; for (let idx1 = 0; idx1 < input.length; idx1++) { arr0[idx1] = parseInt(input[idx1]); } output = arr0; return output; }")
    }
}
