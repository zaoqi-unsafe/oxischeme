// Copyright 2015 Nick Fitzgerald
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Oxischeme is an interpreter, but it is not a naiive AST walking
//! interpreter. In contrast to an AST walking interpreter, syntactic analysis
//! is separated from execution, so that no matter how many times an expression
//! might be evaluated, it is only ever analyzed once.
//!
//! When evaluating a form, first we `analyze` it to derive its semantic
//! `Meaning`. The meanings form an intermediate language containing everything
//! we statically know about the expression, such as whether it is a conditional
//! or a lambda form, or the location of a value bound to a variable name, so
//! that we don't need to compute these things at execution time. After analysis
//! has produced a meaning for the form, the meaning is then interpreted. This
//! approach results in a speed up in the realm of 10 - 50 times faster than
//! simple AST-walking evaluation.
//!
//! In SICP and LiSP, the implementation language is also Scheme, and the
//! meanings are just elegant closures. Because we cannot rely on the host
//! language's GC like they can, we require more fine-grained control of the data
//! and its lifetime. Therefore, we explicitly model all data that can be
//! statically gathered in the `MeaningData` type. Evaluation of each special
//! form is implemented by two things: first, a variant in `MeaningData`, and
//! secondly a `MeaningEvaluatorFn` function that takes the heap, an activation,
//! and the meaning data for that form. The simplest example is quoted forms: we
//! determine the quoted value during analysis and at runtime simply return it.
//!
//!     enum MeaningData {
//!         ...
//!         Quotation(RootedValue),
//!         ...
//!     }
//!
//!     fn evaluate_quotation(heap: &mut Heap,
//!                           data: &MeaningData,
//!                           act: &mut RootedActivationPtr) -> TrampolineResult {
//!         if let MeaningData::Quotation(ref val) = *data {
//!             return Ok(Trampoline::Value(Rooted::new(heap, **val)));
//!         }
//!
//!         panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
//!     }
//!
//!
//! ## References
//!
//! * ["Separating Syntactic Analysis from Execution"](https://mitpress.mit.edu/sicp/full-text/sicp/book/node83.html),
//! chapter 4.1.7 of *Structure and Interpretation of Computer Programs* by
//! Abelson et all
//!
//! * "Fast Interpretation", chapter 6 in *Lisp In Small Pieces* by Christian
//! Queinnec

extern crate test;

use std::cmp::{Ordering};
use std::fmt;
use std::hash;

use environment::{Activation, RootedActivationPtr};
use heap::{Heap, Rooted};
use read::{Location};
use value::{RootedValue, SchemeResult, Value};

/// Evaluate the given form in the global environment.
pub fn evaluate(heap: &mut Heap, form: &RootedValue, location: Location) -> SchemeResult {
    let meaning = try!(analyze(heap, form, location));
    let mut act = heap.global_activation();
    meaning.evaluate(heap, &mut act)
}

/// Evaluate the file at the given path and return the value of the last form.
pub fn evaluate_file(heap: &mut Heap, file_path: &str) -> SchemeResult {
    use read::read_from_file;
    let reader = match read_from_file(file_path, heap) {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Error: evaluate_file could not read {}: {}",
                               file_path,
                               e));
        },
    };

    let mut result = Rooted::new(heap, Value::EmptyList);
    for (location, read_result) in reader {
        let form = try!(read_result);
        result.emplace(*try!(evaluate(heap, &form, location)));
    }

    return Ok(result);
}

/// To optimize tail calls and eliminate the stack frames that would otherwise
/// be used by them, we trampoline thunks in a loop and encode that process in
/// this type.
#[derive(Debug)]
pub enum Trampoline {
    Value(RootedValue),
    Thunk(RootedActivationPtr, Meaning),
}

impl Trampoline {
    /// Keep evaluating thunks until it yields a value.
    pub fn run(self, heap: &mut Heap) -> SchemeResult {
        match self {
            Trampoline::Value(v) => {
                return Ok(v);
            },
            Trampoline::Thunk(act, meaning) => {
                let mut a = act;
                let mut m = meaning;
                loop {
                    match try!(m.evaluate_to_thunk(heap, &mut a)) {
                        Trampoline::Value(v) => {
                            return Ok(v);
                        },
                        Trampoline::Thunk(aa, mm) => {
                            a = aa;
                            m = mm;
                        },
                    }
                }
            }
        }
    }
}

/// Either a `Trampoline`, or a `String` describing the error.
pub type TrampolineResult = Result<Trampoline, String>;

/// The set of data generated by our syntactic analysis pretreatment.
#[derive(Clone, Hash, Debug)]
enum MeaningData {
    /// The quoted value.
    Quotation(RootedValue),

    /// A reference to (i'th activation, j'th binding, original name).
    Reference(u32, u32, String),

    /// Push a new binding to the current activation with the value of the given
    /// meaning.
    Definition(u32, u32, Meaning),

    /// Set the (i'th activation, j'th binding) to the value of the given
    /// meaning.
    SetVariable(u32, u32, Meaning),

    /// Condition, consequent, and alternative.
    Conditional(Meaning, Meaning, Meaning),

    /// Evaluate the first meaning (presumable for side-effects, before
    /// evaluating and returning the second meaning.
    Sequence(Meaning, Meaning),

    /// Arity and body.
    Lambda(u32, Meaning),

    /// Procedure and parameters.
    Invocation(Meaning, Vec<Meaning>),
}

impl fmt::Display for MeaningData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MeaningData::Quotation(ref val) => {
                write!(f, "(quotation {})", **val)
            },
            MeaningData::Reference(i, j, ref name) => {
                write!(f, "(reference {} {} {})", i, j, name)
            },
            MeaningData::Definition(i, j, ref val) => {
                write!(f, "(definition {} {} {})", i, j, val)
            },
            MeaningData::SetVariable(i, j, ref val) => {
                write!(f, "(set-variable {} {} {})", i, j, val)
            },
            MeaningData::Conditional(ref condition,
                                     ref consequent,
                                     ref alternative) => {
                write!(f, "(conditional {} {} {})",
                       condition,
                       consequent,
                       alternative)
            },
            MeaningData::Sequence(ref first, ref second) => {
                write!(f, "(sequence {} {})", first, second)
            },
            MeaningData::Lambda(arity, ref body) => {
                write!(f, "(lambda {} {})", arity, body)
            },
            MeaningData::Invocation(ref procedure, ref arguments) => {
                try!(write!(f, "(invocation {} [", procedure));
                let mut is_first = true;
                for arg in arguments.iter() {
                    try!(write!(f, "{}{}", if is_first { "" } else { " " }, arg));
                    is_first = false;
                }
                write!(f, "])")
            },
        }
    }
}

/// Type signature for the evaulator functions which evaluate only a specific
/// syntactic form.
type MeaningEvaluatorFn = fn(&mut Heap,
                             &MeaningData,
                             &mut RootedActivationPtr) -> TrampolineResult;

impl fmt::Debug for MeaningEvaluatorFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", *self as usize)
    }
}

#[allow(unused_variables)]
fn evaluate_quotation(heap: &mut Heap,
                      data: &MeaningData,
                      act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Quotation(ref val) = *data {
        return Ok(Trampoline::Value(Rooted::new(heap, **val)));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_reference(heap: &mut Heap,
                      data: &MeaningData,
                      act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Reference(i, j, ref name) = *data {
        let val = try!(act.fetch(heap, i, j).ok().ok_or(
            format!("Reference to variable that hasn't been defined: {}", name)));
        return Ok(Trampoline::Value(val));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_definition(heap: &mut Heap,
                       data: &MeaningData,
                       act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Definition(i, j, ref definition_value_meaning) = *data {
        debug_assert!(i == 0,
                      "Definitions should always be in the youngest activation");

        let val = try!(definition_value_meaning.evaluate(heap, act));
        act.define(j, *val);
        return Ok(Trampoline::Value(heap.unspecified_symbol()));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_set_variable(heap: &mut Heap,
                         data: &MeaningData,
                         act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::SetVariable(i, j, ref definition_value_meaning) = *data {
        let val = try!(definition_value_meaning.evaluate(heap, act));
        try!(act.update(i, j, &val).ok().ok_or(
            "Cannot set variable before it has been defined".to_string()));
        return Ok(Trampoline::Value(heap.unspecified_symbol()));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_conditional(heap: &mut Heap,
                        data: &MeaningData,
                        act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Conditional(ref condition,
                                    ref consequent,
                                    ref alternative) = *data {
        let val = try!(condition.evaluate(heap, act));
        return Ok(Trampoline::Thunk(Rooted::new(heap, **act),
                                    if *val == Value::new_boolean(false) {
                                        (*alternative).clone()
                                    } else {
                                        (*consequent).clone()
                                    }));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_sequence(heap: &mut Heap,
                     data: &MeaningData,
                     act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Sequence(ref first, ref second) = *data {
        try!(first.evaluate(heap, act));
        return Ok(Trampoline::Thunk(Rooted::new(heap, **act), second.clone()));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

fn evaluate_lambda(heap: &mut Heap,
                   data: &MeaningData,
                   act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Lambda(arity, ref body) = *data {
        return Ok(Trampoline::Value(
            Value::new_procedure(heap, arity, act, (*body).clone())));
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

pub fn apply_invocation(heap: &mut Heap,
                        proc_val: &RootedValue,
                        args: Vec<RootedValue>) -> TrampolineResult {
    match **proc_val {
        Value::Primitive(primitive) => {
            return primitive.call(heap, args);
        },

        Value::Procedure(proc_ptr) => {
            match proc_ptr.arity.cmp(&(args.len() as u32)) {
                Ordering::Less => {
                    return Err("Error: too many arguments passed".to_string());
                },
                Ordering::Greater => {
                    return Err("Error: too few arguments passed".to_string());
                },
                _ => {
                    let proc_act = proc_ptr.act.as_ref()
                        .expect("Should never see an uninitialized procedure!");
                    let rooted_proc_act = Rooted::new(heap, *proc_act);
                    let body = proc_ptr.body.as_ref()
                        .expect("Should never see an uninitialized procedure!");

                    let new_act = Activation::extend(heap,
                                                     &rooted_proc_act,
                                                     args);
                    return Ok(Trampoline::Thunk(new_act, (**body).clone()));
                },
            }
        },

        _ => {
            return Err(format!("Error: expected a procedure to call, found {}",
                               **proc_val));
        }
    }
}

fn evaluate_invocation(heap: &mut Heap,
                       data: &MeaningData,
                       act: &mut RootedActivationPtr) -> TrampolineResult {
    if let MeaningData::Invocation(ref procedure, ref params) = *data {
        let proc_val = try!(procedure.evaluate(heap, act));
        let args = try!(params.iter().map(|p| p.evaluate(heap, act)).collect());
        return apply_invocation(heap, &proc_val, args);
    }

    panic!("unsynchronized MeaningData and MeaningEvaluatorFn");
}

/// The `Meaning` type is our intermediate language produced by syntactic
/// analysis. It is a triple containing a `MeaningData` variant, its
/// corresponding `MeaningEvaluatorFn`, and the source location this `Meaning`
/// originates from.
#[derive(Debug)]
pub struct Meaning {
    data: Box<MeaningData>,
    evaluator: MeaningEvaluatorFn,
    location: Location,
}

/// ## `Meaning` Constructors
impl Meaning {
    fn new_quotation(form: &RootedValue, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Quotation((*form).clone())),
            evaluator: evaluate_quotation,
            location: location
        }
    }

    fn new_reference(i: u32, j: u32, name: String, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Reference(i, j, name)),
            evaluator: evaluate_reference,
            location: location
        }
    }

    fn new_set_variable(i: u32, j: u32, val: Meaning, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::SetVariable(i, j, val)),
            evaluator: evaluate_set_variable,
            location: location,
        }
    }

    fn new_conditional(condition: Meaning,
                       consquent: Meaning,
                       alternative: Meaning,
                       location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Conditional(condition,
                                                    consquent,
                                                    alternative)),
            evaluator: evaluate_conditional,
            location: location,
        }
    }

    fn new_sequence(first: Meaning, second: Meaning, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Sequence(first, second)),
            evaluator: evaluate_sequence,
            location: location,
        }
    }

    fn new_definition(i: u32, j: u32, defined: Meaning, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Definition(i, j, defined)),
            evaluator: evaluate_definition,
            location: location,
        }
    }

    fn new_lambda(arity: u32, body: Meaning, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Lambda(arity, body)),
            evaluator: evaluate_lambda,
            location: location,
        }
    }

    fn new_invocation(procedure: Meaning, params: Vec<Meaning>, location: Location) -> Meaning {
        Meaning {
            data: Box::new(MeaningData::Invocation(procedure, params)),
            evaluator: evaluate_invocation,
            location: location
        }
    }
}

/// ## `Meaning` Methods
impl Meaning {
    /// Evaluate this form no further than until the next thunk.
    #[inline]
    fn evaluate_to_thunk(&self,
                         heap: &mut Heap,
                         act: &mut RootedActivationPtr) -> TrampolineResult {
        match (self.evaluator)(heap, &*self.data, act) {
            // Add this location to the error message. These stack up and give a
            // backtrace.
            Err(e) => Err(format!("{}:\n{}", self.location, e)),
            ok => ok
        }
    }

    /// Evaluate this form completely, trampolining all thunks until a value is
    /// produced.
    fn evaluate(&self,
                heap: &mut Heap,
                act: &mut RootedActivationPtr) -> SchemeResult {
        let thunk = try!(self.evaluate_to_thunk(heap, act));
        thunk.run(heap)
    }
}

impl Clone for Meaning {
    fn clone(&self) -> Self {
        Meaning {
            data: self.data.clone(),
            evaluator: self.evaluator,
            location: self.location.clone(),
        }
    }
}

impl fmt::Display for Meaning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", *self.data)
    }
}

impl hash::Hash for Meaning {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        let u = self.evaluator as usize;
        u.hash(state);
        self.data.hash(state);
    }
}

/// Either a `Meaning`, or a `String` explaining the error.
pub type MeaningResult = Result<Meaning, String>;

/// The main entry point for syntactic analysis.
pub fn analyze(heap: &mut Heap,
               form: &RootedValue,
               location: Location) -> MeaningResult {
    if form.is_atom() {
        return analyze_atom(heap, form, location);
    }

    let pair = form.to_pair(heap).expect(
        "If a value is not an atom, then it must be a pair.");

    let quote = heap.quote_symbol();
    let if_symbol = heap.if_symbol();
    let begin = heap.begin_symbol();
    let define = heap.define_symbol();
    let set_bang = heap.set_bang_symbol();
    let lambda = heap.lambda_symbol();

    match *pair.car(heap) {
        v if v == *quote     => analyze_quoted(heap, form),
        v if v == *define    => analyze_definition(heap, form),
        v if v == *set_bang  => analyze_set(heap, form),
        v if v == *lambda    => analyze_lambda(heap, form),
        v if v == *if_symbol => analyze_conditional(heap, form),
        v if v == *begin     => analyze_sequence(heap, form),
        _                    => analyze_invocation(heap, form),
    }
}

/// Return true if the form doesn't need to be evaluated because it is
/// "autoquoting" or "self evaluating", false otherwise.
fn is_auto_quoting(form: &RootedValue) -> bool {
    match **form {
        Value::EmptyList    => false,
        Value::Pair(_)      => false,
        Value::Symbol(_)    => false,
        _                   => true,
    }
}

fn analyze_atom(heap: &mut Heap,
                form: &RootedValue,
                location: Location) -> MeaningResult {
    if is_auto_quoting(form) {
        return Ok(Meaning::new_quotation(form, location));
    }

    if let Some(sym) = form.to_symbol(heap) {
        if let Some((i, j)) = heap.environment.lookup(&**sym) {
            return Ok(Meaning::new_reference(i, j, (**sym).clone(), location));
        }

        // This is a reference to a global variable that hasn't been defined
        // yet.
        let (i, j) = heap.environment.define_global((**sym).clone());
        return Ok(Meaning::new_reference(i, j, (**sym).clone(), location));
    }

    return Err(format!("Static error: Cannot evaluate: {}", **form));
}

fn analyze_quoted(heap: &mut Heap, form: &RootedValue) -> MeaningResult {
    if let Ok(2) = form.len() {
        let pair = form.to_pair(heap).unwrap();
        return Ok(Meaning::new_quotation(
            &form.cdr(heap).unwrap().car(heap).unwrap(),
            heap.locate(&pair)));
    }

    let msg = "Static error: Wrong number of parts in quoted form";
    Err(if let Some(pair) = form.to_pair(heap) {
        format!("{}: {}", heap.locate(&pair), msg)
    } else {
        msg.to_string()
    })
}

fn analyze_definition(heap: &mut Heap,
                      form: &RootedValue) -> MeaningResult {
    if let Ok(3) = form.len() {
        let pair = form.to_pair(heap).expect(
            "If len = 3, then form must be a pair");
        let sym = try!(pair.cadr(heap));

        let location = heap.locate(&pair);

        if let Some(str) = sym.to_symbol(heap) {
            let def_value_form = try!(pair.caddr(heap));
            let def_value_meaning = try!(analyze(heap,
                                                 &def_value_form,
                                                 location.clone()));

            let (i, j) = heap.environment.define((**str).clone());
            return Ok(Meaning::new_definition(i, j, def_value_meaning, location));
        }

        return Err(format!("{}: Static error: can only define symbols, found: {}",
                           location,
                           *sym));
    }

    let msg = "Static error: improperly formed definition";
    Err(if let Some(pair) = form.to_pair(heap) {
        format!("{}: {}: {}", heap.locate(&pair), msg, **form)
    } else {
        format!("{}: {}", msg, **form)
    })
}

fn analyze_set(heap: &mut Heap,
               form: &RootedValue) -> MeaningResult {
    if let Ok(3) = form.len() {
        let pair = form.to_pair(heap).expect(
            "If len = 3, then form must be a pair");
        let sym = try!(pair.cadr(heap));

        let location = heap.locate(&pair);

        if let Some(str) = sym.to_symbol(heap) {
            let set_value_form = try!(pair.caddr(heap));
            let set_value_meaning = try!(analyze(heap,
                                                 &set_value_form,
                                                 location.clone()));
            if let Some((i, j)) = heap.environment.lookup(&**str) {
                return Ok(Meaning::new_set_variable(i,
                                                    j,
                                                    set_value_meaning,
                                                    location));
            }

            // This is setting a global variable that isn't defined yet, but
            // could be defined later. The check will happen at evaluation time.
            let (i, j) = heap.environment.define_global((**str).clone());
            return Ok(Meaning::new_set_variable(i,
                                                j,
                                                set_value_meaning,
                                                location));
        }

        return Err(format!("{}: Static error: can only set! symbols, found: {}",
                           location,
                           *sym));
    }

    let msg = "Static error: improperly formed set!";
    Err(if let Some(pair) = form.to_pair(heap) {
        format!("{}: {}: {}", heap.locate(&pair), msg, **form)
    } else {
        format!("{}: {}", msg, **form)
    })
}

fn analyze_lambda(heap: &mut Heap,
                  form: &RootedValue) -> MeaningResult {
    let length = try!(form.len().ok().ok_or_else(|| {
        let msg = "Static error: improperly formed lambda";
        if let Some(pair) = form.to_pair(heap) {
            format!("{}: {}: {}", heap.locate(&pair), msg, **form)
        } else {
            format!("{}: {}", msg, **form)
        }
    }));

    if length < 3 {
        let msg = "Static error: improperly formed lambda";
        return Err(if let Some(pair) = form.to_pair(heap) {
            format!("{}: {}: {}", heap.locate(&pair), msg, **form)
        } else {
            format!("{}: {}", msg, **form)
        })
    }

    let pair = form.to_pair(heap).unwrap();
    let location = heap.locate(&pair);

    let body = pair.cddr(heap)
        .ok().expect("Must be here since length >= 3");

    let mut params = vec!();
    let mut arity = 0;
    let params_form = pair.cadr(heap).ok().expect(
        "Must be here since length >= 3");
    for p in params_form.iter() {
        arity += 1;
        params.push(try!(p.ok().ok_or(format!("{}: Bad lambda parameters: {}",
                                              location,
                                              *params_form))));
    }

    let mut param_names : Vec<String> = try!(params.into_iter().map(|p| {
        let sym = try!(p.to_symbol(heap)
                       .ok_or(format!("{}: Can only define symbol parameters, found {}",
                                      location,
                                      p)));
        Ok((**sym).clone())
    }).collect());

    // Find any definitions in the body, so we can add them to the extended
    // environment.
    let define = heap.define_symbol();
    let mut local_definitions : Vec<String> = body.iter()
        .filter_map(|form_result| {
            if let Ok(form) = form_result {
                if let Some(pair) = form.to_pair(heap) {
                    if pair.car(heap) == define {
                        if let Ok(name) = pair.cadr(heap) {
                            return name.to_symbol(heap).map(|s| (**s).clone())
                        }
                    }
                }
            }

            None
        })
        .collect();

    let mut new_bindings = Vec::with_capacity(param_names.len() + local_definitions.len());
    new_bindings.append(&mut param_names);
    new_bindings.append(&mut local_definitions);

    let body_meaning = try!(heap.with_extended_env(new_bindings, &|heap| {
        make_meaning_sequence(heap, &body)
    }));

    return Ok(Meaning::new_lambda(arity as u32, body_meaning, location));
}

fn analyze_conditional(heap: &mut Heap,
                       form: &RootedValue) -> MeaningResult {
    if let Ok(4) = form.len() {
        let pair = form.to_pair(heap).expect(
            "If len = 4, then form must be a pair");
        let location = heap.locate(&pair);

        let condition_form = try!(pair.cadr(heap));
        let condition_meaning = try!(analyze(heap,
                                             &condition_form,
                                             location.clone()));

        let consequent_form = try!(pair.caddr(heap));
        let consequent_meaning = try!(analyze(heap,
                                              &consequent_form,
                                              location.clone()));

        let alternative_form = try!(pair.cadddr(heap));
        let alternative_meaning = try!(analyze(heap,
                                               &alternative_form,
                                               location.clone()));

        return Ok(Meaning::new_conditional(condition_meaning,
                                           consequent_meaning,
                                           alternative_meaning,
                                           location));
    }

    let msg = "Static error: improperly if expression";
    Err(if let Some(pair) = form.to_pair(heap) {
        format!("{}: {}: {}", heap.locate(&pair), msg, **form)
    } else {
        format!("{}: {}", msg, **form)
    })
}

fn make_meaning_sequence(heap: &mut Heap,
                         forms: &RootedValue) -> MeaningResult {
    if let Some(ref cons) = forms.to_pair(heap) {
        let first_form = cons.car(heap);
        let location = heap.locate(cons);
        let first = try!(analyze(heap, &first_form, location.clone()));

        if *cons.cdr(heap) == Value::EmptyList {
            return Ok(first);
        } else {
            let rest_forms = cons.cdr(heap);
            let rest = try!(make_meaning_sequence(heap, &rest_forms));
            return Ok(Meaning::new_sequence(first, rest, location));
        }
    }

    Err(format!("Static error: improperly formed sequence: {}", **forms))
}

fn analyze_sequence(heap: &mut Heap,
                    form: &RootedValue) -> MeaningResult {
    let forms = try!(form.cdr(heap).ok_or(
        format!("Static error: improperly formed sequence: {}", **form)));
    make_meaning_sequence(heap, &forms)
}

fn make_meaning_vector(heap: &mut Heap,
                       forms: &RootedValue,
                       mut meanings: Vec<Meaning>) -> Result<Vec<Meaning>, String> {
    match **forms {
        Value::EmptyList => Ok(meanings),
        Value::Pair(ref cons) => {
            let car = cons.car(heap);
            let rest = cons.cdr(heap);
            let pair = forms.to_pair(heap).unwrap();
            let location = heap.locate(&pair);
            meanings.push(try!(analyze(heap,
                                       &car,
                                       location)));
            make_meaning_vector(heap, &rest, meanings)
        },
        _ => {
            panic!("Passed improper list to `make_meaning_vector`!");
        }
    }
}

fn analyze_invocation(heap: &mut Heap,
                      form: &RootedValue) -> MeaningResult {
    if let Some(ref cons) = form.to_pair(heap) {
        let location = heap.locate(cons);
        let proc_form = cons.car(heap);
        let proc_meaning = try!(analyze(heap, &proc_form, location.clone()));

        let params_form = cons.cdr(heap);
        let arity = try!(params_form.len().ok().ok_or(
            "Static error: improperly formed invocation".to_string()));
        let params_meaning = try!(make_meaning_vector(
            heap, &params_form, Vec::with_capacity(arity as usize)));

        return Ok(Meaning::new_invocation(proc_meaning, params_meaning, location));
    }

    return Err(format!("Static error: improperly formed invocation: {}", **form));
}

// TESTS -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use heap::{Heap, Rooted};
    use read::{Location};
    use value::{list, Value};

    #[test]
    fn test_eval_integer() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_integer.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(42));
    }

    #[test]
    fn test_eval_boolean() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_boolean.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_boolean(true));
    }

    #[test]
    fn test_eval_quoted() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_quoted.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::EmptyList);
    }

    #[test]
    fn test_eval_if_consequent() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_if_consequent.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(1));
    }

    #[test]
    fn test_eval_if_alternative() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_if_alternative.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(2));
    }

    #[test]
    fn test_eval_begin() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_begin.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(2));
    }

    #[test]
    fn test_eval_variables() {
        let heap = &mut Heap::new();

        let define_symbol = heap.define_symbol();
        let set_bang_symbol = heap.set_bang_symbol();
        let foo_symbol = heap.get_or_create_symbol("foo".to_string());

        let mut def_items = [
            define_symbol,
            foo_symbol,
            Rooted::new(heap, Value::new_integer(2))
        ];
        let def_form = list(heap, &mut def_items);
        evaluate(heap, &def_form, Location::unknown()).ok()
            .expect("Should be able to define");

        let foo_symbol_ = heap.get_or_create_symbol("foo".to_string());

        let def_val = evaluate(heap, &foo_symbol_, Location::unknown()).ok()
            .expect("Should be able to get a defined symbol's value");
        assert_eq!(*def_val, Value::new_integer(2));

        let mut set_items = [
            set_bang_symbol,
            foo_symbol_,
            Rooted::new(heap, Value::new_integer(1))
        ];
        let set_form = list(heap, &mut set_items);
        evaluate(heap, &set_form, Location::unknown()).ok()
            .expect("Should be able to define");

        let foo_symbol__ = heap.get_or_create_symbol("foo".to_string());

        let set_val = evaluate(heap, &foo_symbol__, Location::unknown()).ok()
            .expect("Should be able to get a defined symbol's value");
        assert_eq!(*set_val, Value::new_integer(1));
    }

    #[test]
    fn test_eval_and_call_lambda() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_and_call_lambda.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(5));
    }

    #[test]
    fn test_eval_closures() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_eval_closures.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(1));
    }

    #[test]
    fn test_ref_defined_later() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_ref_defined_later.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(1));
    }

    #[test]
    fn test_set_defined_later() {
        let mut heap = Heap::new();
        let result = evaluate_file(&mut heap, "./tests/test_set_defined_later.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert_eq!(*result, Value::new_integer(5));
    }

    #[test]
    fn test_rooting_bug() {
        let mut heap = Heap::new();
        evaluate_file(&mut heap, "./tests/rooting-bug.scm")
            .ok()
            .expect("Should be able to eval a file.");
        assert!(true, "Should be able to evaluate that file without panicking.");
    }

    #[test]
    fn test_eval_local_definitions() {
        let mut heap = Heap::new();
        match evaluate_file(&mut heap, "./tests/test_eval_local_definitions.scm") {
            Err(msg) => panic!(msg),
            Ok(result) => assert_eq!(*result, Value::new_integer(120)),
        }
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    use super::test::{Bencher};
    use heap::{Heap, Rooted};
    use read::{Location};
    use value::{list, Value};

    #[bench]
    fn bench_iterate_empty_loops(b: &mut Bencher) {
        let mut heap = Heap::new();
        let iter_fn = evaluate_file(&mut heap, "./tests/bench_iterate_empty_loops.scm")
            .ok()
            .expect("Should be able to eval a file.");

        b.iter(|| {
            let mut call_items = [
                iter_fn.clone(),
                Rooted::new(&mut heap, Value::new_integer(10000))
            ];
            let call = list(&mut heap, &mut call_items);
            evaluate(&mut heap, &call, Location::unknown()).ok()
                .expect("Should be able to call our function");
        });
    }

    #[bench]
    fn bench_allocate_cons_cells(b: &mut Bencher) {
        let mut heap = Heap::new();
        let alloc_fn = match evaluate_file(&mut heap, "./tests/bench_allocate_cons_cells.scm") {
            Ok(v) => v,
            Err(msg) => panic!(msg)
        };

        let quote = heap.quote_symbol();
        let empty_list = Rooted::new(&mut heap, Value::EmptyList);

        b.iter(|| {
            let mut call_items = [
                alloc_fn.clone(),
                Rooted::new(&mut heap, Value::new_integer(10000)),
                list(&mut heap, &mut [quote.clone(), empty_list.clone()])
            ];
            let call = list(&mut heap, &mut call_items);
            match evaluate(&mut heap, &call, Location::unknown()) {
                Err(msg) => panic!(msg),
                _ => { }
            };
        });
    }

    #[bench]
    fn bench_eval_metacircular(b: &mut Bencher) {
        let heap = &mut Heap::new();
        let eval_fib_call = match evaluate_file(heap, "./tests/bench_eval_metacircular.scm") {
            Ok(v) => v,
            Err(msg) => panic!(msg)
        };

        b.iter(|| {
            match evaluate(heap, &eval_fib_call.clone(), Location::unknown()) {
                Err(msg) => panic!(msg),
                _ => { },
            };
        });
    }
}
