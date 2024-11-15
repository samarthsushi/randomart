pub mod utils;
use utils::{Colour, LinearCongruentialGenerator};

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    X,                       
    Y,                       
    Random,                  
    Rule(usize),                                    // stores the index of the rule          
    Number(f32),             
    Boolean(bool),           
    Sqrt(Box<Node>),        
    Sin(Box<Node>),
    Cos(Box<Node>),
    Exp(Box<Node>),
    Add(Box<Node>, Box<Node>), 
    Mult(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Modulo(Box<Node>, Box<Node>), 
    Gt(Box<Node>, Box<Node>),   
    Triple(Box<Node>, Box<Node>, Box<Node>), 
    If {
        cond: Box<Node>,     
        then: Box<Node>,    
        elze: Box<Node>,    
    },
    Mix(Box<Node>, Box<Node>, Box<Node>, Box<Node>)
}

impl Node {
    fn eval(&self, x: f32, y: f32) -> Option<f32> {
        match self {
            Node::X => Some(x),
            Node::Y => Some(y),
            Node::Number(value) => Some(*value),
            Node::Random => unreachable!("all Node::Random instances are supposed to be converted into Node::Number during generation"),
            Node::Add(lhs, rhs) => {
                let lhs_val = lhs.eval(x, y)?;
                let rhs_val = rhs.eval(x, y)?;
                Some((lhs_val + rhs_val)/2.0)
            }
            Node::Mult(lhs, rhs) => {
                let lhs_val = lhs.eval(x, y)?;
                let rhs_val = rhs.eval(x, y)?;
                Some(lhs_val * rhs_val)
            }
            Node::Sin(inner) => {
                let val = inner.eval(x, y)?;
                Some(val.sin())
            }
            Node::Cos(inner) => {
                let val = inner.eval(x, y)?;
                Some(val.cos())
            }
            Node::Exp(inner) => {
                let val = inner.eval(x, y)?;
                Some(val.exp())
            }
            Node::Sqrt(inner) => {
                let val = inner.eval(x, y)?;
                Some(val.sqrt().max(0.0)) 
            }
            Node::Div(lhs, rhs) => {
                let lhs_val = lhs.eval(x, y)?;
                let rhs_val = rhs.eval(x, y)?;
                if rhs_val.abs() > 1e-6 { 
                    Some(lhs_val / rhs_val)
                } else {
                    None
                }
            }
            Node::Mix(a, b, c, d) => {
                let a_val = a.eval(x, y)?;
                let b_val = b.eval(x, y)?;
                let c_val = c.eval(x, y)?;
                let d_val = d.eval(x, y)?;
                Some((a_val * c_val + b_val * d_val) / (a_val + b_val + 1e-6))
            }
            Node::Triple(_first, _second, _third) => {
                unreachable!("Node::Triple is only for the Entry rule")
            }
            // todo: enforce boolean values only inside cond
            Node::If { cond, then, elze } => {
                let cond_value = cond.eval(x, y)?; 
                if cond_value > 0.0 { // non zero is true
                    then.eval(x, y)   
                } else {
                    elze.eval(x, y)   
                }
            }
            Node::Gt(lhs, rhs) => {
                let lhs_val = lhs.eval(x, y)?;
                let rhs_val = rhs.eval(x, y)?;
                Some(if lhs_val > rhs_val { 1.0 } else { 0.0 })
            }
            Node::Modulo(lhs, rhs) => {
                let lhs_val = lhs.eval(x, y)?; 
                let rhs_val = rhs.eval(x, y)?; 
                if rhs_val.abs() > 1e-6 { 
                    Some(lhs_val % rhs_val)
                } else {
                    None 
                }
            }
            _ => unreachable!("unexpected Node kind during eval: {:?}", self), 
        }
    }

    pub fn eval_rgb(&self, x: f32, y: f32) -> Colour {
        if let Node::Triple(first, second, third) = self {
            let r = first.eval(x, y).unwrap_or(0.0); 
            let g = second.eval(x, y).unwrap_or(0.0);
            let b = third.eval(x, y).unwrap_or(0.0);
            Colour { r, g, b }
        } else {
            Colour { r: 0.0, g: 0.0, b: 0.0 }
        }
    }
    
    pub fn extract_channels_from_triple(&self) -> (String, String, String) {
        assert!(
            matches!(*self, Node::Triple(_, _, _)),
            "expected the generated node to be a Node::Triple, but found: {:?}",
            self
        );
        match self {
            Node::Triple(left, middle, right) => {
                let r = format!("{:?}", left);
                let g = format!("{:?}", middle);
                let b = format!("{:?}", right);
                (r,g,b)
            }
            _ => {
                unreachable!("assert inside this function would've complained before you came here");
            }
        }
    }
}

#[derive(Clone)]
pub struct GrammarBranch {
    pub node: Box<Node>, 
    pub probability: f32, 
}

#[derive(Clone)]
pub struct GrammarBranches {
    pub alternates: Vec<GrammarBranch>,
}

impl GrammarBranches {
    fn new() -> Self {
        Self {
            alternates: Vec::new(),
        }
    }

    fn add_alternate(&mut self, node: Node, probability: f32) {
        self.alternates.push(GrammarBranch { node: Box::new(node), probability });
    }
}

pub struct Grammar {
    pub rules: Vec<GrammarBranches>, 
    rng: LinearCongruentialGenerator
}

impl Grammar {
    fn add_rule(&mut self, branch: GrammarBranches) {
        self.rules.push(branch);
    }

    pub fn default(seed: u64) -> Self {
        let mut grammar = Self {
            rules: Vec::new(),
            rng: LinearCongruentialGenerator::new(seed),
        };

        // E::= (C, C, C)
        let mut e_branch = GrammarBranches::new();
        e_branch.add_alternate(
            Node::Triple(
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
            ),
            1.0,
        );
        grammar.add_rule(e_branch);

        // C::= A | Add(C, C) | Mult(C, C) | Sin(C) | Cos(C) | Exp(C) | Sqrt(C) | Div(C, C) | Mix(C, C, C, C)
        let mut c_branch = GrammarBranches::new();
        c_branch.add_alternate(Node::Rule(2), 1.0 / 13.0); 
        c_branch.add_alternate(
            Node::Add(
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
            ),
            1.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Mult(
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
            ),
            1.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Sin(Box::new(Node::Rule(1))),
            3.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Cos(Box::new(Node::Rule(1))),
            3.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Exp(Box::new(Node::Rule(1))),
            1.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Sqrt(Box::new(Node::Rule(1))),
            1.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Div(
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
            ),
            1.0 / 13.0,
        );
        c_branch.add_alternate(
            Node::Mix(
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
                Box::new(Node::Rule(1)),
            ),
            1.0 / 13.0,
        );
        grammar.add_rule(c_branch);

        // A ::= x | y | random number in [-1, 1]
        let mut a_branch = GrammarBranches::new();
        a_branch.add_alternate(Node::X, 1.0 / 3.0);
        a_branch.add_alternate(Node::Y, 1.0 / 3.0);
        a_branch.add_alternate(Node::Random, 1.0 / 3.0);
        grammar.add_rule(a_branch);

        grammar  
    
    }

    pub fn build(rules: Vec<GrammarBranches>, seed: u64) -> Self {
        Self { rules, rng: LinearCongruentialGenerator::new(seed) }
    }

    pub fn gen_rule(&mut self, rule: usize, depth: u32) -> Option<Box<Node>> {
        if depth <= 0 {
            return None; 
        }
    
        assert!(rule < self.rules.len(), "invalid rule index");
        let branches = self.rules[rule].clone();
        assert!(!branches.alternates.is_empty(), "no branches available");
    
        let mut node = None;
    
        for _ in 0..100 { 
            let p: f32 = self.rng.next_float(); 
    
            let mut cumulative_probability = 0.0;
            for branch in &branches.alternates {
                cumulative_probability += branch.probability;
                if cumulative_probability >= p {
                    node = self.gen_node(&branch.node, depth - 1);
                    break;
                }
            }
    
            if node.is_some() {
                break; 
            }
        }
    
        node
    }

    fn gen_node(&mut self, node: &Node, depth: u32) -> Option<Box<Node>> {
        match node {
            Node::X | Node::Y | Node::Number(_) | Node::Boolean(_) => Some(Box::new(node.clone())),
    
            Node::Sqrt(inner) |
            Node::Sin(inner) |
            Node::Cos(inner) |
            Node::Exp(inner) => {
                let rhs = self.gen_node(inner, depth)?;
                match node {
                    Node::Sqrt(_) => Some(Box::new(Node::Sqrt(rhs))),
                    Node::Sin(_) => Some(Box::new(Node::Sin(rhs))),
                    Node::Cos(_) => Some(Box::new(Node::Cos(rhs))),
                    Node::Exp(_) => Some(Box::new(Node::Exp(rhs))),
                    _ => unreachable!("{:?} not a unary op", node), 
                }
            }

            Node::Add(lhs, rhs) |
            Node::Mult(lhs, rhs) |
            Node::Modulo(lhs, rhs) |
            Node::Gt(lhs, rhs) |
            Node::Div(lhs, rhs) => {
                let lhs = self.gen_node(lhs, depth)?;
                let rhs = self.gen_node(rhs, depth)?;
                match node {
                    Node::Add(_, _) => Some(Box::new(Node::Add(lhs, rhs))),
                    Node::Mult(_, _) => Some(Box::new(Node::Mult(lhs, rhs))),
                    Node::Modulo(_, _) => Some(Box::new(Node::Modulo(lhs, rhs))),
                    Node::Gt(_, _) => Some(Box::new(Node::Gt(lhs, rhs))),
                    Node::Div(_, _) => Some(Box::new(Node::Div(lhs, rhs))),
                    _ => unreachable!("{:?} not a binary op", node), 
                }
            }
    
            Node::Triple(first, second, third) => {
                let first = self.gen_node(first, depth)?;
                let second = self.gen_node(second, depth)?;
                let third = self.gen_node(third, depth)?;
                Some(Box::new(Node::Triple(first, second, third)))
            }
    
            Node::If { cond, then, elze } => {
                let cond = self.gen_node(cond, depth)?;
                let then = self.gen_node(then, depth)?;
                let elze = self.gen_node(elze, depth)?;
                Some(Box::new(Node::If { cond, then, elze }))
            }
    
            Node::Rule(rule_index) => {
                if let Some(new_depth) = depth.checked_sub(1) {
                    self.gen_rule(*rule_index, new_depth)
                } else {
                    None 
                }
            }
    
            Node::Random => {
                let random_value = self.rng.next_float() * 2.0 - 1.0;
                Some(Box::new(Node::Number(random_value)))
            }
            Node::Mix(a, b, c, d) => {
                let a = self.gen_node(a, depth)?;
                let b = self.gen_node(b, depth)?;
                let c = self.gen_node(c, depth)?;
                let d = self.gen_node(d, depth)?;
                Some(Box::new(Node::Mix(a, b, c, d)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::fnv1a;

    #[test]
    fn it_works() {
        let mut grammar = Grammar::default(fnv1a("samarth kulkarni"));
        let generated_node = grammar.gen_rule(0, 40).unwrap();
        let (r_str, g_str, b_str) = generated_node.extract_channels_from_triple();

        assert_eq!(r_str.as_str(), "Div(Add(Cos(Sin(Number(-0.14320678))), Add(Sin(Cos(Add(Sqrt(Mult(Sin(Sqrt(Exp(Exp(Sin(Cos(Mix(Sin(Cos(Number(0.12987673))), Sin(Mix(Y, X, Y, X)), Cos(Sin(Number(0.3420633))), Mult(Add(Number(-0.36049306), X), Sqrt(Number(-0.599079)))))))))), Exp(Mult(Sin(Sqrt(Exp(Add(Mix(Mult(Sin(Number(-0.06430453)), Cos(Y)), Sin(Sqrt(X)), Add(Sin(Number(-0.77358544)), Cos(X)), Sin(X)), Exp(Exp(Sin(Number(-0.5174536)))))))), Cos(Mult(Sqrt(Cos(Mult(Div(Cos(Number(0.63296735)), Cos(X)), Div(Sin(Y), Y)))), Sin(Sin(X)))))))), Exp(Sin(Mult(Sin(Mult(Mult(Sin(Y), Cos(Cos(Sin(Cos(Cos(Number(-0.37934446))))))), Cos(Exp(Exp(Cos(Mult(Sqrt(Number(-0.059450567)), Sin(Number(-0.36224186))))))))), Sin(Cos(Sqrt(Cos(Mix(Cos(Cos(Sin(X))), Cos(Mult(Cos(Number(-0.48298764)), Sin(Y))), Add(Cos(Sqrt(X)), Add(Mult(X, Y), Sin(Number(-0.26007593)))), Sin(Sin(Div(X, Y)))))))))))))), Number(-0.82172287))), Cos(Cos(Exp(Sin(Exp(Exp(Sin(Sin(Cos(Cos(Sin(Cos(Div(Exp(Sin(Div(X, Y))), Cos(Exp(Cos(X)))))))))))))))))");

        assert_eq!(g_str.as_str(), "Div(Sin(Mult(Cos(Cos(Cos(Cos(Sin(Div(Sin(Mult(Mix(Add(Cos(Mult(Sqrt(Cos(Number(-0.9423096))), Sqrt(Y))), Mix(Mult(Sin(Cos(X)), Div(Cos(Y), Div(X, Y))), Sqrt(Cos(Sin(X))), Y, Exp(Sin(Sin(X))))), Mult(Sin(Mix(Exp(Mult(Number(-0.6048886), X)), Cos(Cos(X)), Cos(Mult(Number(-0.24840617), Y)), Cos(Div(X, Number(-0.9156963))))), Y), Cos(Mix(Div(Add(Mix(Number(0.6295861), Y, X, X), Div(Number(-0.6863682), X)), Add(Mix(Number(-0.8771587), X, Y, Number(-0.08063704)), Cos(X))), Mix(Sqrt(Cos(X)), Exp(Exp(Y)), Cos(Mult(Y, X)), Mult(Sqrt(Number(0.36463416)), Add(X, Y))), Cos(Exp(Sin(Number(-0.5906637)))), Cos(Add(Sin(Y), Mix(Y, Y, Number(-0.04321611), Y))))), Mult(Div(Sin(Sin(Exp(X))), Sin(Cos(Sin(Y)))), Cos(Div(Cos(Div(Y, Y)), Exp(Sqrt(X)))))), Add(Sin(Cos(Sin(Sin(Sin(Number(-0.8430877)))))), Sin(Cos(Cos(Cos(Cos(Y)))))))), Sqrt(Cos(Sin(Number(-0.74280447)))))))))), Sin(Mult(Cos(Mult(Div(Cos(Sin(Sin(Cos(Exp(Sin(Cos(Div(Mix(Number(-0.87980515), Y, Number(-0.13424402), X), Sin(Y))))))))), Cos(Div(X, Sin(Sin(Exp(Sqrt(Cos(Cos(Sin(X)))))))))), Mix(Sqrt(Cos(X)), Exp(Cos(X)), Sin(Sin(Div(Cos(Cos(Cos(Sin(Cos(Cos(Number(-0.41278386))))))), Mult(Add(Exp(Cos(Y)), Mix(Mult(Exp(Cos(Number(0.50737107))), Cos(Sin(X))), Sin(Mix(Sin(X), Sin(Y), Sqrt(X), Y)), Sin(Exp(Exp(Y))), Number(0.6915655))), Exp(Cos(Cos(Sin(Cos(X))))))))), Mult(Sin(Mult(Mix(Exp(Sin(Cos(Exp(Div(X, Number(-0.48297864)))))), Exp(Add(Sqrt(Sin(Sin(Number(-0.73748004)))), Exp(Sin(Sin(Number(-0.58256185)))))), Sin(Mult(Mix(Exp(Div(Y, X)), Exp(Cos(X)), Div(Exp(X), Mix(Y, Y, Number(-0.20308536), Number(0.1147809))), Cos(Mix(X, X, Y, Y))), Add(Mix(Cos(Y), Sqrt(X), Cos(X), Y), Cos(Cos(Number(0.7991773)))))), Mix(Sin(Div(Add(Sin(X), Add(X, Y)), Sin(Sin(Y)))), Cos(Sqrt(Div(Add(Y, Y), Div(Number(-0.0010092854), X)))), Sin(Div(Sin(Div(Number(-0.65613955), Y)), Sin(Cos(Number(0.10976398))))), Mult(Mult(Mult(Sin(Y), Add(Y, Y)), Sin(Sin(X))), Exp(Cos(Exp(Y)))))), Cos(Div(Mix(Exp(Sin(Sin(Y))), Sin(Sin(Div(X, Number(0.68613243)))), Number(-0.36142647), Y), Sin(Div(Sin(Y), Add(Exp(X), Cos(Y)))))))), Mix(Exp(Mix(Sin(Exp(Sin(Div(Cos(Number(0.24319816)), Cos(Number(0.8917732)))))), Add(Cos(Cos(Mix(Cos(Y), Sin(Y), Div(X, Y), Cos(X)))), Add(Sin(Sqrt(Sqrt(Number(-0.40626276)))), Cos(Y))), Sin(Add(Cos(Cos(Mix(X, X, X, X))), Sqrt(Cos(Sqrt(Y))))), Cos(Cos(Mix(Sin(Mix(Y, Number(0.85710704), Number(0.7470633), X)), Y, Sin(Mult(X, Y)), Number(0.77673066)))))), Cos(Sin(Sin(Cos(Cos(Exp(Add(Number(-0.3877203), Y))))))), Cos(Exp(Sin(Exp(Mult(Div(Sin(Y), Div(Y, X)), Cos(Add(Y, Number(-0.6602052)))))))), Exp(Sin(Sqrt(Add(Sin(Cos(Cos(X))), Add(Exp(Div(Number(0.014931679), Number(0.81021035))), Sin(Cos(Y)))))))))))), Sin(Div(Cos(Add(Sin(Sin(Cos(Cos(Cos(Exp(Sin(Sin(Y)))))))), Sin(Sqrt(Cos(Exp(Mult(Sin(Sin(Sin(Y))), Mult(Exp(Exp(Number(0.8102975))), Div(Exp(Y), Cos(Y)))))))))), Mult(Cos(Sin(X)), Cos(Div(Cos(Cos(Sin(Number(0.72053754)))), X))))))))), Cos(Sqrt(X)))");

        assert_eq!(b_str.as_str(), "Mix(Cos(Mult(Div(Mix(Sin(Sin(Sqrt(Sin(Number(0.3495475))))), Mix(Mult(Cos(Mult(Sqrt(Sin(Mult(Mult(Cos(Sin(Number(0.34913957))), Cos(Add(Mult(Sin(X), Sin(Y)), Cos(Exp(Number(0.47822368)))))), Sin(Add(Sqrt(Sin(Sqrt(X))), Sin(Sin(Sin(X)))))))), Cos(Number(-0.028246045)))), Cos(Cos(Sin(Mult(Cos(Sin(Div(Cos(Cos(Cos(X))), Cos(Mult(Cos(X), Sin(X)))))), Sin(Sqrt(Mix(Exp(Cos(Sin(X))), Sin(Div(Sqrt(X), Div(Number(-0.30690622), X))), Sin(Div(Add(Number(-0.03772956), Y), Cos(Y))), Sin(Mult(Cos(Number(0.004562497)), Mix(X, X, X, Y))))))))))), Number(-0.064306915), Cos(Sin(X)), Sin(Cos(Mult(Sin(X), Mult(Div(Div(Sin(Mix(Mult(Mix(Mult(X, Number(-0.109950125)), Sin(X), Sqrt(X), Sin(Y)), Exp(Exp(Y))), Mix(Mult(Div(X, Y), Mult(X, Y)), Cos(Cos(Y)), Sin(Exp(Y)), Add(Sin(Y), Sqrt(Number(0.23427069)))), Sin(Div(Sin(Number(-0.4689204)), Div(Y, X))), Sin(Div(Cos(Number(-0.8679805)), Add(Number(-0.002910018), X))))), Exp(Mult(Add(Cos(Sin(Number(0.102455854))), Div(Mult(Y, Y), Sin(Y))), Cos(Exp(Mult(Y, Y)))))), Y), Cos(Y)))))), Mix(Exp(Sin(Div(Mix(Sin(Mult(Div(Sin(Cos(Mix(Div(X, Y), Cos(Y), Cos(X), Sqrt(Y)))), Add(Exp(Mix(Exp(Number(-0.4068365)), Mix(X, Y, Y, Number(0.755504)), Sin(Number(0.11176741)), Cos(Y))), Sqrt(Div(Mult(X, X), Exp(Number(0.97167003)))))), Mult(Sqrt(Cos(Add(Sin(X), Add(Y, Y)))), Sqrt(Mix(Div(Y, Mult(X, Y)), Div(Add(X, Y), Sin(X)), Mult(Mult(X, Y), Div(X, X)), Add(Cos(Y), Mult(Number(-0.5059426), X))))))), Cos(Add(Add(Sin(Cos(Sin(Cos(Number(0.83246505))))), Mult(Cos(Mix(Div(X, Number(-0.5473858)), Y, Cos(Y), Sin(Number(0.773656)))), Cos(Mult(Mult(Number(0.0025657415), X), Sqrt(Y))))), Mix(Sqrt(Exp(Cos(Cos(X)))), Mult(Cos(Sin(Cos(X))), Sin(Sqrt(Sin(Number(0.1543473))))), Mult(Sqrt(Div(Sin(Y), Mix(Number(-0.7793821), X, Number(0.048429012), Y))), Cos(Sin(Mix(X, Y, Number(-0.23055542), Y)))), Exp(Cos(Cos(Div(X, X))))))), Cos(Div(Sin(Cos(Add(Exp(Sin(X)), Cos(Cos(X))))), Sin(Cos(Div(Cos(Mult(Number(-0.8323845), Y)), Mult(Div(Y, Number(0.24268878)), Cos(X))))))), Div(Cos(Exp(Sin(Number(-0.2940383)))), Exp(Cos(Sin(Sin(Div(Cos(Number(0.99999774)), Sin(X)))))))), Mult(Cos(Sqrt(Sin(Exp(Number(0.7527125))))), Add(Cos(Sin(Add(Exp(Sin(Mix(Y, X, Number(0.52192676), X))), Sin(Exp(Sqrt(Number(-0.25843805))))))), Sin(Mult(Sqrt(Mix(Sqrt(Exp(Y)), Exp(Cos(Y)), Sin(Mix(Y, Y, X, Y)), Cos(Add(Y, X)))), Cos(Cos(Cos(Cos(Y))))))))))), Mult(Mult(Div(Mult(Add(Mult(Sin(Cos(Cos(Sin(Sqrt(Number(0.032577157)))))), Cos(Cos(Div(Cos(Add(Y, X)), Cos(Cos(Number(0.7970786))))))), Sin(Mix(Sin(Sin(Cos(Add(Y, Y)))), Sqrt(Exp(Cos(Sin(X)))), X, Mult(Div(Add(Sqrt(X), Cos(Y)), Div(Sin(X), Sin(Number(0.514042)))), Cos(Cos(Sin(Y))))))), Cos(Cos(Mix(Sin(Sin(Add(Sin(X), Exp(Y)))), Mix(Cos(Mix(Cos(X), Cos(X), Exp(Number(-0.25490594)), X)), Sqrt(Exp(Sin(Y))), Sqrt(Cos(Div(Y, X))), Sqrt(Sin(Div(Y, X)))), Exp(Add(Exp(Cos(Y)), Cos(Cos(Number(-0.14494854))))), Cos(Sqrt(Mult(Cos(Y), Mult(Number(0.81304383), Y)))))))), Sqrt(Cos(Add(Cos(Cos(Sin(Sqrt(Cos(Y))))), Exp(Exp(Cos(Sin(Sin(Y))))))))), Sin(Cos(Add(Div(Mix(X, Exp(X), Cos(Add(Cos(Cos(X)), Mult(Exp(Y), Exp(Y)))), Cos(Y)), Div(X, Sqrt(Sin(Mix(Cos(Number(0.4647845)), Sin(Number(0.025944114)), X, Sqrt(X)))))), Mult(Sin(Add(Sqrt(Sin(Number(0.21214855))), Y)), Exp(Sqrt(Cos(Sqrt(Exp(Y)))))))))), Div(Sin(Mult(Exp(Sin(Exp(Cos(Cos(X))))), Cos(Sin(Div(Div(Exp(Sin(Add(X, Number(0.52490675)))), Mix(Sin(Exp(X)), Mult(Number(0.6354016), Cos(Y)), Sin(Div(Y, X)), Sqrt(Cos(Y)))), Mult(Cos(Add(Mix(Y, X, Number(0.30799115), X), Cos(Y))), Sin(Sin(Div(X, X))))))))), Sin(Div(Cos(Sin(Exp(Exp(Cos(Sin(Sin(Number(0.7638055)))))))), Cos(Div(Number(0.050186753), Sin(Exp(Cos(Div(Exp(Y), Cos(Y))))))))))), Mult(Sin(Number(-0.55209196)), Sin(Exp(Mult(Sin(Exp(Cos(X))), Number(-0.15098155))))), Sin(Sin(Sin(Sin(Div(Cos(Cos(Mult(Cos(Add(Cos(X), Cos(Number(0.1645664)))), Add(Exp(Mult(X, Number(-0.015077531))), Sin(Exp(Number(0.30891848))))))), Y)))))), Sin(Cos(Add(Sin(Div(Sin(Exp(Sqrt(Sin(Cos(Sin(Sin(X))))))), Mult(Cos(Cos(Sin(Mix(Mult(Sin(Number(-0.9504867)), Sin(Number(0.18398857))), Cos(Add(X, Number(0.7596531))), Cos(Y), Div(Cos(X), Sin(Y)))))), Sin(Sin(Y))))), Sin(Cos(Cos(Sin(X)))))))), Add(Mix(Sin(Cos(Mix(Cos(Add(Mult(Mult(Exp(Mult(Div(Cos(Y), Mult(X, X)), Cos(Div(Number(-0.07320893), Y)))), Cos(Mix(Sin(Add(Number(0.97120214), Number(-0.015099168))), Sin(Mult(X, X)), Sin(Cos(Y)), Sin(Cos(Number(0.5798764)))))), Add(Add(Sin(Cos(Cos(Number(-0.5964972)))), Cos(Cos(Y))), Sin(Add(Cos(Cos(Y)), Exp(Div(Number(0.64816475), X)))))), Cos(Exp(Mult(Exp(Sin(Mix(X, X, Y, Y))), Cos(Cos(Exp(Y)))))))), Mix(Sin(Cos(Cos(Exp(Sin(Sin(Sin(Number(0.2052474)))))))), Cos(Add(Sqrt(Cos(Cos(Div(Div(Y, Y), Exp(X))))), Exp(Sin(Sin(Add(Mix(Y, Number(-0.0098260045), X, Y), Exp(X))))))), Mix(Number(0.944718), X, Mix(X, Cos(Sin(Exp(Mix(Mult(X, X), Sin(Y), Cos(X), Sqrt(Number(0.88337076)))))), X, Div(Sqrt(Number(-0.2770806)), Mix(Cos(Cos(Mix(Y, Number(-0.5339681), X, Number(-0.19174957)))), Exp(Exp(Sin(X))), Mix(Number(-0.447838), Mix(Add(Number(-0.8230066), Y), Add(Number(0.7873901), Number(0.64973223)), Cos(Y), Mix(Y, X, X, Y)), Sin(Div(Number(-0.6289691), Number(0.6278701))), Sin(Number(0.708887))), Exp(Div(Mix(Number(0.46761763), Number(-0.22412986), X, X), Sin(Number(-0.18312687))))))), Add(Y, Cos(Y))), Cos(Mult(Sqrt(Cos(Add(Cos(Sin(Number(0.5492463))), Div(Cos(Y), Exp(X))))), Div(Sin(Sin(Exp(Div(Number(-0.038919747), X)))), Cos(Cos(Sin(Exp(Y)))))))), Cos(Exp(Sqrt(Exp(Cos(Div(Y, Exp(Mult(Number(-0.24781406), Number(-0.4888149))))))))), Sin(Cos(Sin(Div(Mult(Sin(Sin(Add(X, Number(0.6833426)))), Mult(Mult(Y, Exp(Number(-0.615209))), Sin(Sin(X)))), Mult(Sqrt(Sqrt(Cos(X))), Cos(Cos(Mult(X, X))))))))))), Cos(Mix(Cos(Sin(Cos(Cos(Cos(Cos(X)))))), Cos(Div(Exp(Cos(Cos(Sqrt(Mix(Sqrt(Mult(Number(0.44387162), Number(-0.5749948))), Add(Exp(Y), Cos(Number(-0.6119069))), Cos(Add(Y, X)), Cos(Sin(X))))))), Number(-0.9596319))), Number(0.23736715), Cos(Sin(Sin(Mult(X, Exp(Sin(Sin(Div(Mult(Y, X), Sin(Number(-0.052057028)))))))))))), Cos(Sin(Sin(Sin(Div(Mix(Cos(X), Sqrt(Sin(Sin(Sin(Cos(Y))))), Sin(Sin(Sqrt(Exp(Cos(Number(0.8709527)))))), Number(-0.92868245)), Mult(Sin(X), Sin(Mult(Div(Cos(Mult(X, Number(0.4355904))), Sin(Cos(Y))), Sin(Div(Sin(Number(0.2802844)), Mix(Number(-0.83142304), Number(0.617736), X, Number(0.7274498)))))))))))), Div(Sin(Cos(Mix(Cos(Div(Sqrt(Exp(Mult(Cos(Exp(X)), Exp(Sqrt(Y))))), Exp(Mult(Sin(Cos(Sqrt(X))), Mult(Cos(Sin(Number(-0.61841726))), Mult(Sin(Y), Sqrt(Number(0.41551423)))))))), Cos(Mix(Cos(Cos(Cos(Mix(Number(-0.07645905), Exp(Y), Mix(X, Y, Number(-0.58819205), Number(0.5839802)), Cos(X))))), Exp(Y), Add(Mult(Mix(Sqrt(Sin(Number(-0.46542597))), Add(X, Add(Y, Y)), Add(Sin(X), Div(Y, Number(-0.21287918))), Sin(Cos(X))), Sin(X)), Add(Mult(Mult(Sin(X), Cos(Number(-0.43943208))), Div(Exp(Y), Cos(Number(0.45012224)))), Cos(Sin(Add(Number(0.7323346), X))))), Cos(Mix(Mix(Cos(Mult(Y, X)), Mult(X, Sin(X)), Cos(Cos(Y)), Sin(Mix(Number(0.20066547), Y, X, Number(-0.09153783)))), Mult(Add(Exp(Y), Mix(Number(0.32048595), Y, X, Y)), Mix(Div(Y, Y), Cos(X), Div(X, X), Mult(Y, X))), Sin(Sin(Cos(X))), Y)))), Mix(Cos(Sin(Div(Cos(Sin(Sqrt(X))), Cos(Y)))), Sin(Cos(Sqrt(Cos(Mix(Cos(Y), Sin(X), Y, Mult(Y, Y)))))), Exp(Sin(Mix(Mix(Cos(Cos(Y)), Div(Sin(Number(0.14871824)), Sin(Y)), Add(Exp(X), Add(Y, Y)), Number(-0.94822216)), Mix(Sin(Exp(Y)), Sin(Div(Y, Y)), Cos(Div(Y, X)), Y), Cos(Add(Number(0.40875208), Exp(Number(-0.042478323)))), Cos(Sin(Number(-0.031902194)))))), Exp(Y)), Cos(Sin(Mix(Sin(Cos(Number(-0.10499954))), Cos(Cos(Exp(Exp(Y)))), Exp(Sin(Cos(Cos(Y)))), Cos(Add(Y, Add(Sin(X), Cos(X)))))))))), Mult(Cos(Sin(Add(Sin(Sqrt(Sqrt(Cos(Cos(Sin(X)))))), Div(Sin(Cos(Div(Sin(Number(0.22065699)), Sin(Cos(Number(0.74862957)))))), Number(0.7524036))))), Number(-0.37149942)))), Exp(Cos(Cos(Y))))), Cos(Sin(Sin(Y))))), Mix(Sin(Cos(Div(Sin(Div(Sin(Cos(Y)), Sin(Mix(Mix(Sin(Exp(Exp(Cos(Mix(Mult(Cos(X), Mix(Number(0.5934701), X, Y, X)), Cos(Sin(Y)), Sin(Sin(X)), Add(Div(Y, Y), Y)))))), Sin(Cos(Sin(Sin(Cos(Y))))), Cos(Sin(Sin(X))), Sqrt(X)), Mix(Mix(X, Sin(Cos(Add(Mix(Exp(Exp(Y)), X, Exp(Cos(Y)), Sin(Sin(Number(-0.9456918)))), Div(Mix(Sqrt(Y), Mix(X, X, Number(-0.78852916), Number(-0.17267483)), Mix(Y, Y, X, X), Cos(Y)), Sin(Cos(Number(-0.8741473))))))), Exp(Cos(Mult(Sin(Div(Exp(Number(0.22395623)), Add(Number(-0.3532501), Y))), Cos(Cos(Sin(X)))))), Mix(Sin(Exp(Cos(Sin(Mult(X, Y))))), Exp(Mult(Sin(Sin(Sqrt(X))), Add(Sqrt(Sin(Number(-0.18035722))), Mult(Cos(X), Cos(X))))), Cos(Sin(Cos(Mult(Mix(Number(0.86577857), Y, Y, X), Cos(Number(-0.6594358)))))), Y)), Y, Cos(Cos(Number(0.5229447))), Mix(Sin(Div(Cos(Sin(Cos(Sqrt(X)))), Cos(Sin(Sin(Sqrt(Number(-0.103687644))))))), Sin(Mix(Exp(Sin(Add(Add(X, X), Sin(Y)))), Cos(Number(-0.28975892)), Sin(Exp(Sin(Cos(Number(0.98677516))))), Exp(Cos(Cos(Exp(Number(-0.470249))))))), Add(Mix(Cos(Sin(Div(Mult(X, X), Cos(Number(-0.5741714))))), Sin(Add(Sin(Div(X, Y)), Y)), Cos(Mix(Sqrt(Div(Y, X)), Cos(Sin(Number(-0.96921295))), Add(Mult(X, X), Exp(Number(0.6879387))), Sin(Cos(Number(-0.06764853))))), Sin(Sin(Cos(Cos(Number(-0.4187088)))))), Add(Cos(Cos(Div(Cos(X), Div(X, X)))), Div(Cos(Sin(Sin(X))), Cos(Mult(Mult(Y, Y), Sqrt(Number(0.3062575))))))), Sin(Sin(Cos(Sin(X)))))), Cos(Sqrt(Cos(Cos(Exp(Number(0.24036384)))))), Cos(Add(Sin(Sin(Number(-0.5450039))), Sin(X))))))), Mix(Sqrt(Cos(Sin(Sin(Sin(Y))))), Div(Cos(Sin(Y)), Cos(Add(Sqrt(Cos(Cos(Exp(Cos(Cos(Number(-0.16056913))))))), Y))), Mix(Cos(Sqrt(X)), Cos(Mult(Sin(Div(Number(0.33157992), Sin(Sin(Sin(Sin(Div(Exp(X), Div(X, X)))))))), Cos(Add(Sin(Mult(Add(Add(Mult(Div(Number(-0.2154535), Y), Cos(Y)), Sin(Cos(Y))), Div(Sin(Cos(Y)), Sin(Sin(Y)))), Cos(Cos(Cos(Add(Y, Number(0.14503872))))))), Mix(Exp(Sin(Cos(Mult(Mix(X, Number(-0.4516794), X, Number(0.89927673)), X)))), Cos(Number(0.65038323)), Sin(Cos(Div(Mult(Mix(X, X, Y, Number(-0.791076)), Div(Number(-0.07804692), Number(0.7185346))), Add(Div(Number(0.38050663), Number(-0.57676524)), Cos(X))))), Sqrt(Sin(Mix(Exp(Div(Y, X)), Sin(Cos(Y)), Cos(Exp(Y)), Cos(Mult(Y, Number(-0.18346393))))))))))), Cos(Cos(Cos(Sqrt(Sqrt(Sin(Div(Exp(Exp(Exp(Y))), Cos(Add(Sin(X), Mult(X, Number(-0.7582235))))))))))), Sqrt(Cos(Add(Sin(Cos(Number(-0.48211753))), Sin(Cos(Sin(Exp(Cos(Exp(Exp(Y))))))))))), Add(Mix(Number(-0.59193516), Sin(Sin(Exp(Sqrt(Div(Mult(Mix(Sin(Sin(X)), Cos(Add(X, X)), Cos(Sin(X)), Sin(Y)), Exp(Sin(Sqrt(Number(-0.37427878))))), Cos(Sin(Sin(Cos(X))))))))), Exp(Add(Mult(Sin(Y), Exp(Cos(Sin(Add(Y, Y))))), Sqrt(Sin(Sin(Mix(Cos(Sqrt(Mult(Y, X))), Mult(Cos(Number(-0.74629235)), Cos(Exp(Y))), Div(Cos(Cos(Y)), Sqrt(Y)), Sin(Sin(Sqrt(Number(-0.4606545)))))))))), Sin(Cos(X))), Mix(X, Div(Mult(Sqrt(Cos(Cos(Sin(Sqrt(Exp(X)))))), Number(0.06035924)), Cos(Y)), Sin(Sqrt(Mult(Cos(Cos(X)), Mult(Cos(Cos(Sin(Exp(Sin(Y))))), Add(Cos(Cos(Sin(Cos(X)))), Cos(Cos(Sqrt(Add(Number(-0.71127474), Y))))))))), Sqrt(Mult(Sin(Sqrt(Cos(Sin(Mult(Mix(Div(X, Number(-0.13324112)), Add(X, X), Add(Y, Number(-0.51584125)), Sin(X)), Y))))), Sin(Sin(Div(Exp(Mult(Div(Cos(X), Cos(Y)), Add(Sin(Y), Exp(X)))), Cos(Sin(Mix(Add(Y, X), Sin(Number(-0.67880726)), Div(Y, X), Add(Number(-0.57505476), X))))))))))))))), Cos(Cos(Cos(Sin(Sin(Sin(Sin(Add(Sin(Mix(Y, X, Cos(Sin(Cos(Mix(Sin(Y), Add(Number(0.65507483), Y), Sin(Number(-0.9894132)), Sin(Y))))), Cos(Add(Mult(Cos(Cos(Number(0.19391227))), Cos(Mix(Y, Number(0.19417083), X, X))), Add(Sin(Sin(X)), Cos(Mult(X, X))))))), Sin(Sqrt(Cos(Cos(Mult(X, Sin(Exp(Number(0.64459777)))))))))))))))), Sin(Mix(Y, Cos(Sin(Cos(Mix(Sin(Cos(Mult(Sin(Sin(Mix(Mix(X, Mult(Cos(Y), Sin(Y)), Add(X, Exp(Y)), Add(Sqrt(Number(0.48107958)), Mult(X, X))), Add(Cos(Exp(X)), Sin(Sin(Y))), Div(Mult(Cos(Y), Exp(Y)), Div(Cos(X), Sin(Y))), Sin(Exp(Div(Y, X)))))), Y))), Sin(Exp(Sin(Sin(Sin(Mult(Exp(Sin(Mix(X, X, Y, Y))), Sin(Div(Sin(X), Exp(X))))))))), Div(Mult(Div(Cos(Mix(Cos(Sin(Sin(Cos(Number(-0.19622135))))), Y, Y, Sin(Sin(Mix(Sin(X), Cos(Y), Sin(X), Add(Number(-0.07689148), Number(0.19692588))))))), Exp(Div(Div(Sin(Cos(Sin(Number(0.004932761)))), Add(Y, Cos(Mix(Number(0.84725547), Y, Y, Number(0.1958046))))), Cos(Sin(Sqrt(Cos(X))))))), Mix(Sqrt(Number(-0.04027897)), X, Sin(Sin(Sin(Exp(Cos(Div(X, Number(-0.9133847))))))), Div(Exp(Cos(Cos(Sin(Sin(Y))))), Sqrt(Mix(Number(-0.08670664), X, Div(Mult(Cos(Y), Exp(Number(-0.5727257))), Cos(Div(X, Y))), Cos(Add(Cos(Number(0.7861527)), Add(X, X)))))))), Exp(Sin(Sin(Div(Sin(Add(Add(Cos(Y), Cos(Number(-0.17198682))), Cos(Sqrt(Number(-0.9646359))))), Sin(Sqrt(Mult(Sin(Y), Add(Number(0.9888084), X))))))))), Exp(Mult(Sin(Sqrt(Div(Add(Mult(Div(Sqrt(Number(0.3537501)), Cos(Number(0.3001616))), Y), Add(Cos(Cos(Number(-0.7373171))), Sin(Y))), Sqrt(Cos(Sqrt(Cos(X))))))), X)))))), Sin(Sqrt(Cos(Sin(Cos(Cos(Div(Mult(Mult(Div(X, Mult(Sin(Add(Number(0.6632433), Y)), Mix(Cos(Y), Add(Number(-0.49911207), X), Cos(X), Cos(Number(-0.81166095))))), Exp(X)), Sin(Mix(Sqrt(Cos(Cos(Y))), Cos(Exp(Mult(Number(-0.21213251), Y))), Mix(Exp(Cos(Y)), Exp(Mult(Y, Y)), Div(Mult(X, Y), Cos(Number(0.038701415))), Cos(Sin(Number(0.83962953)))), Div(Cos(Sin(Number(-0.63057184))), Cos(Cos(Y)))))), Mult(Sin(Mult(Add(Div(Cos(Number(-0.38629252)), Mix(X, Y, Y, X)), Add(Cos(Number(0.9263333)), Mult(Y, Number(-0.62272906)))), Cos(Exp(Sin(Y))))), Sin(Mult(Div(Sin(Add(X, Number(-0.68700856))), Sin(Div(X, X))), Sin(Sin(Sin(X))))))))))))), Add(Cos(Mix(Mult(Sin(Mix(Exp(Add(Cos(Sqrt(Cos(Add(Sqrt(X), Number(0.3543434))))), Cos(Sqrt(Add(Cos(Sin(Y)), Sin(Sqrt(Number(-0.5828701)))))))), Exp(Div(Cos(Add(Cos(Cos(Exp(Y))), Sin(Cos(Cos(Number(0.056019068)))))), Sin(Sin(Exp(Sqrt(Number(-0.24269682))))))), Sin(Mix(Exp(Cos(Cos(Div(Mix(Number(-0.3485222), X, Y, Y), Cos(X))))), Y, Sin(Cos(Exp(Sin(Sin(Y))))), Sin(Mult(Cos(Mix(Div(X, Y), Cos(X), Exp(Number(0.33842492)), Add(X, Y))), Sin(Add(Cos(Y), Div(Y, Number(0.9116055)))))))), Sqrt(Cos(Sin(Sin(Cos(Mult(Div(Number(-0.80568063), Number(0.6912154)), Sin(X))))))))), Cos(Mult(Sqrt(Sin(Cos(Cos(Cos(Mix(Mix(X, Number(-0.2530492), Number(-0.052922726), Y), Exp(Number(0.13713562)), Cos(Y), Mix(Number(-0.2388376), Number(-0.9496919), Number(-0.3593567), Number(-0.62272346)))))))), Sin(Mix(Cos(Cos(X)), Mix(Sin(Number(0.81479096)), Exp(Div(Cos(Add(Y, Y)), Y)), Cos(Sqrt(Add(Mult(X, Y), Cos(X)))), Sin(Sin(Exp(Cos(X))))), Sin(Cos(Cos(Mix(Sin(Number(-0.9390233)), Div(Number(0.033102274), Number(-0.9761268)), Number(0.61303484), Cos(X))))), Mix(Sin(Y), Sin(Sin(Sin(Exp(X)))), Sin(Exp(Y)), Add(Sin(Cos(Sin(X))), Div(Sin(Div(Number(0.87449336), Number(-0.5972011))), Cos(Mix(Number(0.35999036), Number(-0.8804752), Y, Y)))))))))), Sin(Sqrt(Cos(Y))), X, Mult(Sqrt(Sqrt(Exp(X))), Mult(Sin(Cos(Sin(Add(Cos(Cos(Sin(Sqrt(Number(-0.050644934))))), Div(Add(Cos(Cos(Number(-0.6347277))), Cos(Exp(Y))), Sin(Cos(Div(Y, Y)))))))), X)))), Cos(Number(0.1404506))))), Cos(Sqrt(Sin(Sin(Sqrt(Mult(Mix(Add(Mult(Sin(Sin(Sin(Add(Sin(Sin(X)), Sqrt(Cos(X)))))), Sin(Sqrt(Sin(Exp(Sin(Add(Number(0.41177797), Number(-0.8347945)))))))), Cos(Cos(Exp(Cos(Sin(Add(Mult(Number(0.1145432), Number(-0.08312094)), Sin(Y)))))))), Mult(Mult(Sqrt(Cos(Div(Sqrt(Mult(Exp(Y), Exp(Y))), Mult(Mix(Mult(Number(0.2157197), X), Sin(Y), Sqrt(Y), Mult(Number(-0.37020934), X)), Exp(Sin(X)))))), Sin(Mix(Cos(Add(Cos(Mix(Y, X, Number(-0.2876191), X)), X)), Cos(Add(Exp(Cos(X)), Sin(Sqrt(X)))), Sin(Mult(Sin(Sqrt(Y)), Add(Sin(X), Mix(X, Y, Y, Y)))), Exp(Cos(Sin(Sin(Y))))))), Mix(Sin(Cos(Mix(Cos(Add(Sin(Y), Cos(Y))), Sin(Sin(Y)), Cos(Sin(Sin(Number(0.3515575)))), Sqrt(Number(-0.35058212))))), Sin(Mix(Sin(Number(0.11452341)), Sin(Cos(Sin(Div(Number(-0.244488), X)))), Sin(Cos(Cos(Sin(Y)))), Sin(Sin(Add(Sin(Number(-0.7298197)), Mult(X, Y)))))), Cos(Sin(Number(0.17072177))), X)), Mult(Y, Exp(Add(X, Add(Cos(Mult(Cos(Number(-0.69769335)), Exp(Mult(Y, X)))), Mult(Cos(Mult(Mult(Number(0.120052814), Number(-0.11341834)), Div(Y, Number(0.4210354)))), Exp(Mix(Sin(Y), Div(Y, Y), Cos(Number(0.4464395)), Cos(X)))))))), Sqrt(Exp(Cos(Sin(Sin(Sin(Sin(Cos(Y))))))))), Cos(Cos(Sin(Sin(Cos(Add(Div(Cos(Cos(Number(-0.6713137))), Cos(Mix(Y, Y, Number(0.7867204), Number(-0.81227815)))), X))))))))))))), Sin(Cos(Sin(Sin(Sin(Mix(Cos(Cos(Div(Sin(Sin(Mix(Sqrt(Div(Cos(Cos(X)), Sin(Add(Y, X)))), Cos(Mix(Sqrt(Cos(X)), Mult(Number(-0.85110164), Exp(Y)), Add(Mix(X, Y, Y, Y), Cos(X)), Div(Mix(Number(0.40844977), Y, Number(-0.33420676), Number(-0.29338688)), Div(X, Number(0.32558942))))), Cos(Cos(Sin(Sin(Number(-0.509833))))), Div(Mult(Cos(Mult(X, Y)), Cos(Sin(Number(0.9400883)))), Mix(Sin(Mult(Y, Y)), Exp(Mult(X, X)), Mult(Mix(Number(-0.51896966), Number(-0.21270794), X, X), Mix(Y, X, Y, Number(0.36942196))), Sqrt(Sin(Number(0.07482827)))))))), Cos(Cos(Div(Add(Cos(X), Sin(Cos(Sin(X)))), Sin(Sqrt(Mix(Cos(Number(-0.71121716)), Sin(Y), Mix(X, Number(-0.70973265), Number(-0.8225516), Y), Sin(Number(0.019764543))))))))))), X, Exp(Add(X, Add(Sqrt(Mult(Sin(Sin(Sin(Sin(Mult(X, X))))), Cos(Cos(Add(Mix(Cos(Y), Cos(X), Cos(Number(0.65322125)), Sin(X)), Cos(Cos(Number(-0.85839695)))))))), Mult(Add(Mix(Cos(Sin(Y)), Sqrt(Sqrt(Exp(Cos(X)))), Cos(Cos(Mix(Add(Number(0.72308826), Number(0.65712035)), Sin(Y), Sin(Y), Div(Y, Y)))), Cos(Cos(Sqrt(Y)))), Sqrt(Sqrt(Sin(Sqrt(Sin(Number(0.88247526))))))), Exp(Exp(Cos(Sin(Sin(Add(Number(0.9214587), Number(-0.33683997))))))))))), Cos(Cos(Sin(Sin(Cos(Cos(Number(-0.32478553))))))))))))), Mix(Cos(Add(Y, Exp(Exp(Mult(Add(Cos(Sin(Cos(Add(Add(Add(Sin(Add(Cos(X), Mix(X, X, X, Number(-0.5659093)))), Cos(Sin(Cos(Number(-0.24634367))))), Add(Sin(Cos(Sin(Y))), Cos(Cos(Sin(X))))), Y)))), Sin(Cos(Sqrt(Sqrt(Sqrt(Y)))))), Cos(Sin(Cos(Sin(Cos(Sin(Sin(Cos(Add(Number(0.9574189), Cos(Number(-0.5263555)))))))))))))))), Sqrt(Sqrt(Cos(Sin(Sin(Cos(Sin(Sin(Div(Add(Sqrt(Mult(Sin(Div(Sin(Number(0.44231117)), Exp(X))), Sin(X))), Exp(Div(Mult(Sqrt(Y), Mult(Exp(Number(-0.5057694)), Sqrt(Number(0.1781007)))), Sin(Sin(Sin(X)))))), Add(Div(Div(Sin(Add(Sin(X), Sin(Number(0.39516246)))), Cos(Cos(Sin(Number(0.5324826))))), X), Sin(Sin(Sqrt(Sin(Mix(Y, X, Y, Number(-0.08513969)))))))))))))))), Mult(Cos(Exp(X)), Div(Y, Mix(Sin(Cos(Sin(Number(-0.8000852)))), Sqrt(Sqrt(Div(Cos(Exp(Div(X, Add(Sqrt(Sin(Sin(Sqrt(Sqrt(X))))), Sin(Exp(Div(Mix(Cos(Y), Cos(Number(-0.53036886)), Cos(Y), Cos(Y)), Mult(Cos(Y), Sin(Number(-0.3313613)))))))))), Mult(Cos(Y), Sin(Mix(Number(0.12652946), Sin(Sin(Sin(X))), Cos(Mult(Sin(Sin(Div(Sin(Y), Sin(Y)))), Mix(Div(Sin(Cos(X)), Sin(X)), Mix(Cos(Cos(Number(0.53654385))), Add(Sin(X), Sin(Y)), Sin(Div(X, Y)), Sin(Sin(Number(0.82944465)))), Sin(Sin(Mult(Number(0.32553697), Y))), Cos(Sin(Div(X, Number(0.15278625))))))), Mix(Number(-0.34005803), Mult(Cos(Mult(Cos(Mix(Y, Y, X, Number(-0.6285552))), Sqrt(Div(X, X)))), Cos(Cos(Cos(Cos(Number(0.063637614)))))), Mix(Div(Div(Cos(Cos(Y)), Mult(Cos(Number(0.86710715)), Cos(X))), Add(Exp(X), Sin(Cos(X)))), Div(Mult(Div(Exp(Y), Sqrt(Y)), Div(X, Mult(X, X))), Sqrt(Sqrt(Sqrt(Y)))), Cos(Sin(Number(-0.5801364))), Mix(Cos(Sqrt(Cos(Y))), Sin(Cos(Mix(Y, Number(-0.7305194), X, X))), Cos(Cos(Cos(Number(-0.36816114)))), Y)), Mix(Exp(Mult(Cos(Cos(Y)), Exp(Mix(X, X, X, Number(0.911221))))), Sqrt(Sin(Exp(Exp(X)))), Sin(Cos(Cos(Sqrt(X)))), Mult(Cos(Exp(Mult(Y, X))), Cos(Mix(Div(Y, X), Number(-0.16653204), Sin(Y), Cos(Y)))))))))))), X, Exp(Add(Exp(Div(Sqrt(Cos(Cos(Sin(Add(Cos(Cos(Add(X, X))), X))))), Sqrt(Add(Number(0.5408325), Sin(Div(Cos(Sin(Mult(Cos(Number(-0.0048134923)), Cos(Number(0.29158628))))), X)))))), Cos(Cos(Sqrt(Sin(Add(Sin(Sin(Cos(Y))), Cos(Div(Sin(Sqrt(Cos(Number(-0.18105245)))), Cos(Sin(Sin(X))))))))))))))), Mix(Mult(Sin(Y), Sin(Mult(Sin(Mult(Sin(Mix(Div(Sin(Cos(Mix(Add(Sin(Sin(Number(-0.49157864))), Sin(Sin(Y))), Cos(Cos(Cos(Number(-0.033185363)))), Sqrt(Sin(Cos(Number(0.5512737)))), Mult(Sin(Mult(Y, X)), Sin(Sin(Number(-0.092920005))))))), Div(Sqrt(Mix(Mix(Sin(Number(-0.58630705)), Sin(Sqrt(Number(-0.85498893))), Cos(Sqrt(Y)), Exp(Cos(Number(-0.59395146)))), Sin(Exp(Cos(X))), Cos(Div(Cos(Number(0.82890105)), Mix(Y, Number(0.54296076), X, Y))), Cos(Mult(Div(X, Y), Div(Number(0.11185467), Y))))), Sqrt(Sqrt(Sin(Exp(Cos(Number(-0.78901434)))))))), Exp(Sin(Mix(Add(Add(Cos(Sqrt(Y)), Sin(Sqrt(Y))), Sqrt(Sin(Div(Y, Number(-0.1351577))))), Number(-0.09597337), Cos(Cos(Sin(Sin(X)))), Mult(Mix(Exp(Sqrt(Number(-0.8210181))), Add(Number(0.96317446), Exp(X)), Cos(Exp(Number(-0.30183554))), Sin(Cos(X))), Mix(Number(0.24030519), Exp(Div(Number(-0.9082313), Y)), Sin(Div(Y, Y)), Cos(Mix(Y, Number(-0.9880708), X, Y))))))), Mult(Add(Sin(Mult(Exp(Mult(Add(X, X), Sin(X))), Sin(Sqrt(Cos(Y))))), Cos(Sin(Cos(Mix(Div(X, Y), Add(X, Number(0.39153206)), Add(Y, Y), Mix(Y, Number(-0.91434324), X, Y)))))), Cos(Cos(Sin(Sqrt(Sin(Mult(X, Number(0.6091267)))))))), Y)), X)), Exp(Cos(Div(Sin(X), Exp(Add(Mult(Add(Mult(Mix(Sqrt(Sin(X)), Sin(Mix(Y, Number(-0.52658355), Y, X)), Cos(Sin(Number(-0.3398602))), Div(Cos(X), Add(X, X))), Mult(Mult(Div(X, Y), Sqrt(Y)), Div(Sin(Y), Sin(X)))), Mix(Add(Mult(Cos(Number(0.5483179)), Exp(Y)), Sin(Add(X, Y))), Sin(Div(Add(X, Number(-0.500489)), Cos(X))), Cos(Sin(Add(Number(-0.5626701), Number(-0.768016)))), Div(Sin(Exp(X)), Sqrt(Cos(Number(0.68145275)))))), Sin(Div(Exp(Sqrt(Add(Number(-0.55919194), X))), Sqrt(Add(Div(Number(-0.6945194), X), Sin(Number(-0.8119772))))))), Cos(Y))))))))), Cos(Sin(Sin(Mult(Sqrt(Mult(Exp(Cos(Cos(Cos(Mix(Sin(Sqrt(Div(Y, Y))), Mult(Mix(Cos(X), Number(-0.9674282), Cos(Number(0.18193519)), Cos(Y)), Number(-0.93708736)), X, Cos(Mult(Exp(X), Cos(X)))))))), Cos(Sqrt(Sqrt(Sin(Sin(Add(Cos(Sin(Number(-0.77558005))), Cos(Add(X, X)))))))))), Mix(Mix(Exp(Exp(Sqrt(Add(Cos(Cos(Mult(Mix(Number(-0.547691), Y, Number(-0.007953167), Y), Y))), Add(Cos(Number(-0.5697737)), Sin(Cos(Exp(X)))))))), Mult(Cos(Cos(Cos(Sin(Div(Div(Mix(Y, Number(-0.98098034), Y, X), Mix(Number(-0.30729312), X, Number(-0.38762206), Number(0.43075))), Sqrt(Mix(X, Y, Number(0.77707875), Y))))))), Exp(Mix(Exp(Sin(Div(Cos(Sin(X)), Exp(Sqrt(Y))))), Sin(Add(Mix(Sin(Cos(Y)), Sin(Mix(Number(0.09861851), Number(0.41210186), X, Y)), Mult(Cos(Y), Div(Number(0.514251), Number(0.36669517))), Sin(Cos(Y))), Sin(Cos(Cos(X))))), Sin(Cos(Cos(Sin(Mix(Y, X, X, Number(-0.22689337)))))), Div(Cos(Mix(Mult(Y, Sin(X)), Sin(Add(X, X)), Exp(Mult(X, Y)), Number(0.94753397))), Exp(Sin(Sin(Sin(Number(0.16472459))))))))), Div(Sin(Exp(X)), Cos(Sin(Sqrt(Sqrt(Mult(Add(Exp(X), Mix(Number(-0.88163716), Y, Number(-0.8088156), X)), Cos(Mult(Number(0.39522815), Y)))))))), Cos(Add(Cos(Cos(Sin(Div(Mult(Div(X, X), Mult(Number(0.100788474), Number(-0.8276571))), Exp(Cos(X)))))), Cos(Cos(Cos(Exp(Cos(Exp(Number(0.9137466)))))))))), Exp(Sqrt(Sqrt(Add(Sin(Cos(Mult(X, X))), Number(-0.56119907))))), Sqrt(Sin(Sin(Cos(Mult(Add(Add(Sqrt(Exp(Y)), Add(Cos(Number(-0.5982443)), Sin(Number(-0.8906658)))), Mix(Number(0.18970656), Sin(Sin(X)), Cos(Mult(Number(0.64319396), X)), Cos(Sin(Number(-0.9670738))))), Sin(Sqrt(Mult(Sin(Y), Number(0.47086084))))))))), Cos(X)))))), Sin(Div(Sqrt(Cos(Mult(Y, Sin(Sin(Sin(Cos(Cos(Mult(Cos(Mult(Sin(Y), Sin(X))), X))))))))), Cos(Cos(Mix(Sin(Mix(Sin(Exp(Sin(Div(Mix(Sin(Cos(Y)), Sin(Div(Y, X)), Cos(Mult(Y, Y)), Cos(Cos(Y))), Sin(Sqrt(Div(Y, X))))))), Mix(Sin(Div(Sqrt(Sin(Mix(Y, Div(Y, Y), Cos(X), Mix(Number(0.729542), Number(0.33759356), X, X)))), Sqrt(Mix(Exp(Sin(Y)), Cos(X), Sqrt(Cos(X)), Sin(Sin(Y)))))), Mult(Cos(Mix(Mix(Sqrt(Cos(X)), Mult(Add(Number(-0.74272144), Y), Sin(X)), Mix(Sin(Y), Sqrt(Number(-0.4827276)), Cos(X), Add(Y, Y)), Y), Cos(Sin(Cos(X))), Mix(Mix(Sqrt(X), Sin(Number(0.3298148)), Sin(Number(-0.053871572)), Sin(X)), Cos(Cos(Y)), Add(Exp(Number(-0.81930643)), Exp(X)), Sin(Exp(X))), Mult(Sin(Sin(Number(-0.85651624))), Sin(Sin(Y))))), Mix(Cos(Cos(Add(Sin(Number(-0.8444094)), Sin(Y)))), Sqrt(Sin(Add(Cos(X), Exp(Number(0.35632575))))), Sin(Cos(Sqrt(Add(Number(0.8418726), Number(-0.26727647))))), Cos(Cos(Div(Cos(Number(-0.31411916)), Add(Number(-0.9008402), Number(-0.27474898))))))), Sin(Div(Cos(Mix(Sin(Sin(Number(0.7573323))), Mix(Cos(Y), Sin(Number(-0.7425834)), Sin(Y), Mult(X, X)), Sin(Exp(Number(0.59653795))), Mix(Sin(X), Sqrt(Number(-0.7789163)), Div(X, X), Y))), X)), Exp(X)), Cos(Mult(Mix(Mult(Cos(Cos(Div(Y, Y))), Cos(Cos(Sin(X)))), Sqrt(Mult(Add(Sqrt(X), Number(-0.87333417)), Sin(Add(Number(0.20148325), Y)))), Cos(Add(Cos(Cos(X)), Cos(Mix(X, Y, X, X)))), Number(0.97270644)), Sqrt(Cos(X)))), Cos(Div(Exp(Cos(Sin(Mix(Sin(Number(0.34279788)), Cos(Number(-0.6548697)), Sin(Number(-0.25879836)), Cos(Y))))), Sin(Div(Cos(Sin(Mult(Y, X))), Sin(Div(Div(Y, Number(0.68324757)), Add(X, Number(0.87821496)))))))))), Add(Cos(Mult(Cos(Div(Add(Sin(Sin(Add(Number(-0.055552423), Number(0.99940765)))), Mult(Y, Sin(Mix(Y, X, Number(-0.9506445), X)))), Sin(Mult(Sin(Cos(Number(0.41392815))), Cos(Cos(Y)))))), Sqrt(Mix(Mult(Mix(Sin(Sin(X)), Mix(Cos(Y), Cos(Number(0.05619228)), Cos(Number(0.96714926)), Exp(Number(0.36962056))), Add(Cos(X), Cos(Number(-0.6130491))), Mix(Cos(Number(0.27031553)), Mult(Y, Y), Mix(X, Number(0.5066961), Y, Number(0.56643534)), Cos(Y))), Cos(Cos(Cos(Number(-0.798253))))), Exp(Add(Exp(Sin(Number(0.023844123))), Sin(Sin(Number(-0.035259962))))), Mult(Cos(Sin(Sin(Number(0.18251252)))), Number(-0.8906218)), Cos(Add(Sqrt(Cos(Y)), Add(Div(Y, Y), Sin(X)))))))), Mix(Sqrt(Sqrt(Cos(Sqrt(Sin(Div(Mix(X, Number(0.37046874), Y, Number(-0.7258955)), Add(X, X))))))), Sin(Sqrt(Number(0.4627918))), Mix(Cos(Sin(Sin(Mix(Sin(Cos(Number(0.66513515))), Exp(Cos(Y)), Sin(Cos(X)), Cos(Sin(Number(-0.42014897))))))), Cos(X), Cos(Exp(Mult(Add(Sin(Y), Sin(Sin(Number(-0.678146)))), Sin(Cos(Mix(Number(-0.66386217), Number(-0.91416013), Number(-0.643108), Y)))))), Exp(Div(Sin(Sin(Cos(Mult(Number(0.46010458), Number(0.24866962))))), Add(Sin(Cos(Cos(X))), Sqrt(Div(Cos(Y), Exp(Number(0.55982554)))))))), Cos(Add(Mult(Cos(Y), Sin(Cos(Exp(Sin(X))))), Exp(Cos(X)))))), Sin(Mult(Mix(Cos(Add(Number(-0.9908572), Sin(Cos(Mult(Y, Exp(Y)))))), Cos(Cos(Mix(Add(Sin(Sin(Number(-0.6659081))), Sin(Mult(X, X))), Sin(Cos(Mult(Number(0.7922491), X))), Cos(Cos(Div(Number(0.55245924), Y))), Number(0.26839328)))), Sin(Sin(X)), Mult(X, Sin(Add(Mult(Sin(Sin(Y)), Cos(Sin(Y))), Mult(Sin(Sin(X)), Cos(Exp(Y))))))), Mult(Cos(Sin(Exp(Sqrt(Sin(Cos(Number(-0.027014256))))))), Sqrt(Cos(Add(Exp(Sqrt(Mult(Number(0.5093312), Number(0.7001902)))), Sqrt(X))))))), Cos(Mix(Mix(Sin(Div(Div(Sin(Sqrt(Sin(Number(-0.49456346)))), Add(Cos(Cos(X)), Mix(Sin(Number(-0.31392235)), Mix(Number(0.81482446), Number(-0.26293957), Number(-0.043800473), Number(-0.87839764)), Sin(Y), Cos(Number(-0.5087792))))), Div(Exp(Sin(Sin(Y))), Sqrt(Mix(Sqrt(Y), Div(X, X), Sin(X), Exp(X)))))), Div(Add(Mix(Add(Mult(Mult(Y, Y), Sin(Number(0.38485897))), Sin(Cos(X))), Exp(Sin(Sqrt(Y))), Cos(Mult(Mult(Y, X), Sin(Number(0.801288)))), Exp(Sin(Sqrt(X)))), Sin(Exp(Sqrt(Cos(Number(0.43515754)))))), X), Sin(Mix(Cos(Add(Sin(Sin(X)), Cos(Sin(Number(-0.41117322))))), Exp(Mix(Cos(Mult(X, X)), Mult(Sin(X), Sqrt(X)), Add(Cos(Y), Y), Sin(Mix(Number(-0.25734514), Number(-0.66918135), Number(-0.94133484), X)))), Cos(Div(Exp(Cos(Y)), Sin(Exp(X)))), Cos(Cos(Y)))), Sin(Mix(Div(Div(Sin(Sin(X)), Number(-0.4480347)), Sin(Exp(Number(-0.8372743)))), Exp(Cos(Sin(Mult(Y, Y)))), Add(Cos(Div(Cos(X), Exp(X))), Cos(Cos(Sqrt(Y)))), Add(Div(Mix(Cos(Number(0.87464654)), Sin(Number(-0.4076411)), Sin(Y), Sqrt(Number(0.52001953))), Cos(Mix(Number(0.0205127), Y, X, Y))), Sin(Sin(Cos(Number(0.37853062)))))))), Cos(Sin(Add(Mult(Sqrt(Sqrt(Cos(X))), Sin(Y)), Cos(Cos(Div(Sqrt(X), Exp(Y))))))), Mix(Mult(Sin(Mix(Cos(Sqrt(Div(X, Number(-0.6577972)))), Sqrt(Sin(Cos(Y))), Cos(Sqrt(Add(Number(0.33436704), X))), Mix(Sin(Sqrt(Number(0.122047305))), Mult(Cos(Number(0.43963623)), Cos(Y)), Sqrt(Mix(X, X, Y, Number(-0.18602538))), Sqrt(Mult(Number(0.81209207), X))))), Cos(Sqrt(Sqrt(Sqrt(Add(Y, Y)))))), X, Cos(Div(Sin(Sin(Mult(Cos(Number(-0.9063332)), Cos(Number(0.68846583))))), Number(0.65864503))), Sin(Cos(Exp(Sin(Cos(Cos(X))))))), X))))))), Sqrt(Y))))");
    }

    #[test]
    #[should_panic(expected = "expected the generated node to be a Node::Triple")]
    fn test_extract_channels_from_triple_panics_on_invalid_variant() {
        let invalid_node = Node::X;
        invalid_node.extract_channels_from_triple();
    }
}

