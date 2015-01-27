var searchIndex = {};
searchIndex['oxischeme'] = {"items":[[0,"","oxischeme","A Scheme implementation, in Rust."],[5,"repl","","Start a Read -> Evaluate -> Print loop."],[5,"main","","Given no arguments, start the REPL. Otherwise, treat each argument as a file\npath and read and evaluate each of them in turn."],[0,"environment","","The implementation of the Scheme environment binding symbols to values."],[3,"Activation","oxischeme::environment","An `Activation` represents a runtime instance of a lexical block (either a\nlambda or the global top-level)."],[3,"Environment","","The `Environment` represents what we know about bindings statically, during\nsyntactic analysis."],[6,"ActivationPtr","","A pointer to an `Activation` on the heap."],[6,"RootedActivationPtr","","A rooted pointer to an `Activation` on the heap."],[11,"extend","","Extend the given `Activation` with the values supplied, resulting in a\nnew `Activation` instance.",0],[11,"fetch","","Fetch the j'th variable from the i'th lexical activation.",0],[11,"update","","Set the j'th variable from the i'th lexical activation to the given\nvalue.",0],[11,"define","","Define the j'th variable of this activation to be the given value.",0],[11,"hash","","",0],[11,"default","","",0],[11,"trace","","",0],[11,"fmt","","",0],[11,"to_gc_thing","","",1],[11,"new","","Create a new `Environemnt`.",2],[11,"extend","","Extend the environment with a new lexical block with the given set of\nvariables.",2],[11,"pop","","Pop off the youngest lexical block.",2],[11,"define","","Define a variable in the youngest block and return the coordinates to\nget its value from an activation at runtime.",2],[11,"define_global","","Define a global variable and return its activation coordinates.",2],[11,"lookup","","Get the activation coordinates associated with the given variable name.",2],[0,"eval","oxischeme","Oxischeme is an interpreter, but it is not a naiive AST walking\ninterpreter. In contrast to an AST walking interpreter, syntactic analysis\nis separated from execution, so that no matter how many times an expression\nmight be evaluated, it is only ever analyzed once."],[3,"Meaning","oxischeme::eval","The `Meaning` type is our intermediate language produced by syntactic\nanalysis. It is a pair containing a `MeaningData` variant and its\ncorresponding `MeaningEvaluatorFn`."],[4,"Trampoline","","To optimize tail calls and eliminate the stack frames used by them, we\ntrampoline thunks in a while loop and encode that process in this type."],[13,"Value","","",3],[13,"Thunk","","",3],[5,"evaluate","","Evaluate the given form in the global environment."],[5,"evaluate_file","","Evaluate the file at the given path and return the value of the last form."],[5,"analyze","","The main entry point for syntactic analysis."],[6,"TrampolineResult","","Either a `Trampoline`, or a `String` describing the error."],[6,"MeaningResult","","Either a `Meaning`, or a `String` explaining the error."],[11,"fmt","","",3],[11,"fmt","","",4],[11,"clone","","",4],[11,"fmt","","",4],[11,"hash","","",4],[0,"heap","oxischeme","The `heap` module provides memory management for our Scheme implementation."],[3,"Arena","oxischeme::heap","An arena from which to allocate `T` objects from."],[3,"ArenaPtr","","A pointer to a `T` instance in an arena."],[3,"Rooted","","A smart pointer wrapping the pointer type `T`. It keeps its referent rooted\nwhile the smart pointer is in scope to prevent dangling pointers caused by a\ngarbage collection within the pointers lifespan. For more information see\nthe module level documentation about rooting."],[3,"Heap","","The scheme heap and GC runtime, containing all allocated cons cells,\nactivations, procedures, and strings (including strings for symbols)."],[12,"environment","","The static environment.",5],[4,"GcThing","","The union of the various types that are GC things."],[13,"Cons","","",6],[13,"String","","",6],[13,"Activation","","",6],[13,"Procedure","","",6],[6,"StringPtr","","A pointer to a string on the heap."],[6,"RootedStringPtr","","A rooted pointer to a string on the heap."],[6,"IterGcThing","","An iterable of `GcThing`s."],[7,"DEFAULT_CONS_CAPACITY","","The default capacity of cons cells."],[7,"DEFAULT_STRINGS_CAPACITY","","The default capacity of strings."],[7,"DEFAULT_ACTIVATIONS_CAPACITY","","The default capacity of activations."],[7,"DEFAULT_PROCEDURES_CAPACITY","","The default capacity of procedures."],[8,"ToGcThing","","A trait for types that can be coerced to a `GcThing`."],[10,"to_gc_thing","","Coerce this value to a `GcThing`.",7],[8,"Trace","","The `Trace` trait allows GC participants to inform the collector of their\nreferences to other GC things."],[10,"trace","","Return an iterable of all of the GC things referenced by this structure.",8],[11,"new","","Create a new `Arena` with the capacity to allocate the given number of\n`T` instances.",9],[11,"capacity","","Get this heap's capacity for simultaneously allocated cons cells.",9],[11,"is_full","","Return true if this arena is at full capacity, and false otherwise.",9],[11,"allocate","","Allocate a new `T` instance and return a pointer to it.",9],[11,"sweep","","Sweep the arena and add any reclaimed objects back to the free list.",9],[11,"hash","","",10],[6,"Target","",""],[11,"deref","","",10],[11,"deref_mut","","",10],[11,"fmt","","",10],[11,"eq","","Note that `PartialEq` implements pointer object identity, not structural\ncomparison. In other words, it is equivalent to the scheme function\n`eq?`, not the scheme function `equal?`.",10],[11,"fmt","","",11],[11,"hash","","",11],[11,"new","","Create a new `Rooted<T>`, rooting the referent.",11],[11,"emplace","","Unroot the current referent and replace it with the given referent,\nwhich then gets rooted.",11],[11,"to_gc_thing","","",11],[6,"Target","",""],[11,"deref","","",11],[11,"deref_mut","","",11],[11,"drop","","",11],[11,"clone","","",11],[11,"eq","","",11],[11,"to_gc_thing","","",12],[11,"new","","Create a new `Heap` with the default capacity.",5],[11,"with_arenas","","Create a new `Heap` using the given arenas for allocating cons cells and\nstrings within.",5],[11,"allocate_cons","","Allocate a new cons cell and return a pointer to it.",5],[11,"allocate_string","","Allocate a new string and return a pointer to it.",5],[11,"allocate_activation","","Allocate a new `Activation` and return a pointer to it.",5],[11,"allocate_procedure","","Allocate a new `Procedure` and return a pointer to it.",5],[11,"collect_garbage","","Perform a garbage collection on the heap.",5],[11,"add_root","","Explicitly add the given GC thing as a root.",5],[11,"drop_root","","Unroot a GC thing that was explicitly rooted with `add_root`.",5],[11,"increase_gc_pressure","","Apply pressure to the GC, and if enough pressure has built up, then\nperform a garbage collection.",5],[11,"global_activation","","Get the global activation.",5],[11,"with_extended_env","","Extend the environment with a new lexical block containing the given\nvariables and then perform some work before popping the new block.",5],[11,"get_or_create_symbol","","Ensure that there is an interned symbol extant for the given `String`\nand return it.",5],[11,"quote_symbol","","",5],[11,"if_symbol","","",5],[11,"begin_symbol","","",5],[11,"define_symbol","","",5],[11,"set_bang_symbol","","",5],[11,"unspecified_symbol","","",5],[11,"lambda_symbol","","",5],[11,"fmt","","",6],[11,"eq","","",6],[11,"ne","","",6],[11,"hash","","",6],[11,"from_string_ptr","","Create a `GcThing` from a `StringPtr`.",6],[11,"from_cons_ptr","","Create a `GcThing` from a `ConsPtr`.",6],[11,"from_procedure_ptr","","Create a `GcThing` from a `ProcedurePtr`.",6],[11,"from_activation_ptr","","Create a `GcThing` from an `ActivationPtr`.",6],[11,"trace","","",6],[0,"primitives","oxischeme","Implementation of primitive procedures."],[5,"define_primitives","oxischeme::primitives",""],[6,"PrimitiveFunction","","The function signature for primitives."],[0,"read","oxischeme","Parsing values."],[3,"Read","oxischeme::read","`Read` iteratively parses values from the input `Reader`."],[5,"read_from_bytes","","Create a `Read` instance from a byte vector."],[5,"read_from_string","","Create a `Read` instance from a `String`."],[5,"read_from_str","","Create a `Read` instance from a `&str`."],[5,"read_from_file","","Create a `Read` instance from the file at `path_name`."],[11,"new","","Create a new `Read` instance from the given `Reader` input source.",13],[11,"get_result","","Get the results of parsing thus far. If there was an error parsing, a\ndiagnostic message will be the value of the error.",13],[6,"Item","",""],[11,"next","","",13],[0,"value","oxischeme","Scheme value implementation."],[3,"Cons","oxischeme::value","A cons cell is a pair of `car` and `cdr` values. A list is one or more cons\ncells, daisy chained together via the `cdr`. A list is \"proper\" if the last\n`cdr` is `Value::EmptyList`, or the scheme value `()`. Otherwise, it is\n\"improper\"."],[3,"Procedure","","User defined procedures are represented by their body and a pointer to the\nactivation that they were defined within."],[12,"arity","","",14],[12,"body","","",14],[12,"act","","",14],[3,"Primitive","","A primitive procedure, such as Scheme's `+` or `cons`."],[3,"ConsIterator","","An iterator which yields `Ok` for each value in a cons-list and finishes\nwith `None` when the end of the list is reached (the scheme empty list\nvalue) or `Err` when iterating over an improper list."],[4,"Value","","`Value` represents a scheme value of any type."],[13,"EmptyList","","The empty list: `()`.",15],[13,"Pair","","The scheme pair type is a pointer to a GC-managed `Cons` cell.",15],[13,"String","","The scheme string type is a pointer to a GC-managed `String`.",15],[13,"Symbol","","Scheme symbols are also implemented as a pointer to a GC-managed\n`String`.",15],[13,"Integer","","Scheme integers are represented as 64 bit integers.",15],[13,"Boolean","","Scheme booleans are represented with `bool`.",15],[13,"Character","","Scheme characters are `char`s.",15],[13,"Procedure","","A user-defined Scheme procedure is a pointer to a GC-managed\n`Procedure`.",15],[13,"Primitive","","A primitive Scheme procedure is just a pointer to a `Primitive` type\nfunction pointer.",15],[5,"list","","A helper utility to create a cons list from the given values."],[6,"ConsPtr","","A pointer to a cons cell on the heap."],[6,"RootedConsPtr","","A rooted pointer to a cons cell on the heap."],[6,"ProcedurePtr","","A pointer to a `Procedure` on the heap."],[6,"RootedProcedurePtr","","A rooted pointer to a `Procedure` on the heap."],[6,"RootedValue","",""],[6,"SchemeResult","","Either a Scheme `RootedValue`, or a `String` containing an error message."],[11,"eq","","",16],[11,"ne","","",16],[11,"hash","","",16],[11,"default","","Do not use this method, instead allocate cons cells on the heap with\n`Heap::allocate_cons` and get back a `ConsPtr`.",16],[11,"car","","Get the car of this cons cell.",16],[11,"cdr","","Get the cdr of this cons cell.",16],[11,"set_car","","Set the car of this cons cell.",16],[11,"set_cdr","","Set the cdr of this cons cell.",16],[11,"trace","","",16],[11,"to_gc_thing","","",17],[11,"default","","",14],[11,"trace","","",14],[11,"hash","","",14],[11,"to_gc_thing","","",18],[11,"eq","","",19],[11,"hash","","",19],[11,"call","","",19],[11,"fmt","","",19],[11,"fmt","","",15],[11,"eq","","",15],[11,"ne","","",15],[11,"hash","","",15],[11,"new_integer","","Create a new integer value.",15],[11,"new_boolean","","Create a new boolean value.",15],[11,"new_character","","Create a new character value.",15],[11,"new_pair","","Create a new cons pair value with the given car and cdr.",15],[11,"new_procedure","","Create a new procedure with the given parameter list and body.",15],[11,"new_primitive","","",15],[11,"new_string","","Create a new string value with the given string.",15],[11,"new_symbol","","Create a new symbol value with the given string.",15],[11,"car","","Assuming this value is a cons pair, get its car value. Otherwise, return\n`None`.",15],[11,"cdr","","Assuming this value is a cons pair, get its cdr value. Otherwise, return\n`None`.",15],[11,"is_pair","","Return true if this value is a pair, false otherwise.",15],[11,"is_atom","","Return true if this value is an atom, false otherwise.",15],[11,"to_symbol","","Coerce this symbol value to a `StringPtr` to the symbol's string name.",15],[11,"to_pair","","Coerce this pair value to a `ConsPtr` to the cons cell this pair is\nreferring to.",15],[11,"to_procedure","","Coerce this procedure value to a `ProcedurePtr` to the `Procedure` this\nvalue is referring to.",15],[11,"to_integer","","Coerce this integer value to its underlying `i64`.",15],[11,"len","","Assuming that this value is a proper list, get the length of the list.",15],[11,"iter","","Iterate over this list value.",15],[11,"to_gc_thing","","",15],[11,"fmt","","Print the given value's text representation to the given writer. This is\nthe opposite of `Read`.",15],[6,"Item","",""],[11,"next","","",20],[11,"cddr","","",16],[11,"cdddr","","",16],[11,"cadr","","",16],[11,"caddr","","",16],[11,"cadddr","","",16]],"paths":[[3,"Activation"],[6,"ActivationPtr"],[3,"Environment"],[4,"Trampoline"],[3,"Meaning"],[3,"Heap"],[4,"GcThing"],[8,"ToGcThing"],[8,"Trace"],[3,"Arena"],[3,"ArenaPtr"],[3,"Rooted"],[6,"StringPtr"],[3,"Read"],[3,"Procedure"],[4,"Value"],[3,"Cons"],[6,"ConsPtr"],[6,"ProcedurePtr"],[3,"Primitive"],[3,"ConsIterator"]]};
initSearch(searchIndex);
