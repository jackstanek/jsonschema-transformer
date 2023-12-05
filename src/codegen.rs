use std::{fmt::{Display, format}, sync::Arc};

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
        varname
    }

    fn new_obj(&mut self, prefix: &str) -> Level {
        let varname = self.new_var(prefix);
        let obj = Level::Var(varname);
        self.varstack.push(obj.clone());
        obj
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
        use IR::*;

        let mut frags = Vec::new();

        for op in it {
            match op {
                G2G(from, to) => {
                    if let Some(frag) = self.generate_ground_to_ground(from, to) {
                        frags.push(frag)
                    }
                }
                PushArr => {
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
                PopArr => {
                    let popvar = self.poptop();
                    if let Arr(var, _) = popvar {
                        frags.push("}".to_string());
                        frags.push(format!("{} = {};", self.output_path(), var));
                    } else {
                        panic!("PopArr instruction executed but top of stack was not arr");
                    }
                }
                PushKey(k) => {
                    self.varstack.push(Key(k));
                }
                PopKey => {
                    if let Some(top) = self.varstack.pop() {
                        if let Key(_) = top {
                        } else {
                            panic!("PopKey instruction executed but top of stack was not a key")
                        }
                    }
                }
                PushObj => {
                    let var = self.new_obj("obj");
                    frags.push(format!("let {} = {{}};", var));
                }
                PopObj => {
                    let top = self.poptop();
                    frags.push(format!("{} = {};", self.output_path(), top))
                }
                Abs(k) => {
                    frags.push(format!(
                        "{} = {{\"{}\": {} }};",
                        self.output_path(),
                        k,
                        self.input_path()
                    ));
                }
                Copy => frags.push(format!(
                    "{} = structuredClone({});",
                    self.output_path(),
                    self.input_path()
                )),
                Inv => todo!(),
                Extr(_) => todo!(),
            }
        }

        //TODO: Use some AST representation instead of raw strings.
        let mut indent: usize = 1;
        let code: String = frags
            .into_iter()
            .map(|frag| {
                if frag.ends_with('}') {
                    indent -= 1;
                }
                let line = format!("{}{}", " ".repeat(4 * indent), frag);
                if frag.ends_with('{') {
                    indent += 1;
                }
                line
            })
            .collect::<Vec<String>>()
            .join("\n");
        format!(
            "function({}) {{\n{}\n    return {};\n}}",
            self.arg, code, self.retvar,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Ground;
    use IR::*;

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
            .generate(vec![G2G(Ground::String, Ground::Num)].into_iter());
        assert_eq!(
            code,
            "\
function(input) {
    output = parseInt(input);
    return output;
}"
        )
    }

    #[test]
    fn test_js_parse_int_in_obj() {
        let code = JSCodegen::new("input", "output").generate(
            vec![
                PushObj,
                PushKey(Arc::new("foo".to_string())),
                G2G(Ground::String, Ground::Num),
                PopKey,
                PopObj,
            ]
            .into_iter(),
        );
        assert_eq!(
            code,
            "\
function(input) {
    let obj0 = {};
    obj0.foo = parseInt(input.foo);
    output = obj0;
    return output;
}"
        )
    }

    #[test]
    fn test_js_parse_int_in_array_in_obj() {
        let code = JSCodegen::new("input", "output").generate(
            vec![
                PushObj,
                PushKey(Arc::new("foo".to_string())),
                PushArr,
                G2G(Ground::String, Ground::Num),
                PopArr,
                PopKey,
                PopObj,
            ]
            .into_iter(),
        );
        assert_eq!(
            code,
            "\
function(input) {
    let obj0 = {};
    let arr1 = [];
    for (let idx2 = 0; idx2 < input.foo.length; idx2++) {
        arr1[idx2] = parseInt(input.foo[idx2]);
    }
    obj0.foo = arr1;
    output = obj0;
    return output;
}"
        )
    }

    #[test]
    fn test_push_arr() {
        let code = JSCodegen::new("input", "output")
            .generate(vec![PushArr, G2G(Ground::String, Ground::Num), PopArr].into_iter());
        assert_eq!(code, "\
function(input) {
    let arr0 = [];
    for (let idx1 = 0; idx1 < input.length; idx1++) {
        arr0[idx1] = parseInt(input[idx1]);
    }
    output = arr0;
    return output;
}")
    }

    #[test]
    fn test_abs_key() {
        let code = JSCodegen::new("input", "output")
            .generate(vec![PushArr, Abs(Arc::new("foo".to_string())), PopArr].into_iter());
        assert_eq!(code, "\
function(input) {
    let arr0 = [];
    for (let idx1 = 0; idx1 < input.length; idx1++) {
        arr0[idx1] = {\"foo\": input[idx1] };
    }
    output = arr0;
    return output;
}")
    }

    #[test]
    fn test_del_key() {
        let code = JSCodegen::new("input", "output").generate(vec![PushObj, PopObj].into_iter());
        assert_eq!(
            code,
            "\
function(input) {
    let obj0 = {};
    output = obj0;
    return output;
}"
        )
    }
}
