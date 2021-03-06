extern crate antlr_parser;
#[macro_use]
extern crate clap;
extern crate grammartec;
extern crate ron;
extern crate serde_json;

use grammartec::context::Context;
use grammartec::context::SerializableContext;
use grammartec::newtypes::NTermID;
use grammartec::tree::TreeLike;

use clap::{App, Arg};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::Path;

fn main() {
    //Parse parameters
    let matches = App::new("generator")
        .about("Generate strings using a grammar. This can also be used to generate a corpus")
        .arg(Arg::with_name("grammar_path")
             .short("g")
             .value_name("GRAMMAR")
             .takes_value(true)
             .required(true)
             .help("Path to grammar"))
        .arg(Arg::with_name("tree_depth")
             .short("t")
             .value_name("DEPTH")
             .takes_value(true)
             .required(true)
             .help("Size of trees that are generated"))
        .arg(Arg::with_name("number_of_trees")
             .short("n")
             .value_name("NUMBER")
             .takes_value(true)
             .help("Number of trees to generate [default: 1]"))
        .arg(Arg::with_name("store")
             .short("s")
             .help("Store output to files. This will create a folder called corpus containing one file for each generated tree."))
        .arg(Arg::with_name("dumb")
             .short("d")
             .help("Don't use fancy calculations to generate trees (dumb mode)"))
        .arg(Arg::with_name("verbose")
             .short("v")
             .help("Be verbose"))
        .get_matches();


    let grammar_path = matches.value_of("grammar_path")
        .expect("grammar_path is a required parameter")
        .to_string();
    let tree_depth = value_t!(matches, "tree_depth", usize)
        .expect("tree_depth is a requried parameter");
    let number_of_trees = value_t!(matches, "number_of_trees", usize)
        .unwrap_or(1);
    let store = matches.is_present("store");
    let dumb = matches.is_present("dumb");
    let verbose = matches.is_present("verbose");

    let mut ctx;
    let serialized_context_path = grammar_path.clone() + ".gfc";
    let mut maybe_serialized_context = None;
    //Calculate string of grammar file
    let mut gf = File::open(grammar_path.clone()).expect("cannot open grammar file");
    let mut content = String::new();
    gf.read_to_string(&mut content)
        .expect("cannot read grammar file");
    let mut s = DefaultHasher::new();
    content.hash(&mut s);
    let hash = s.finish();
    //Deserialize saved context if the granmmar did not change (hash value still the same)
    if Path::new(&serialized_context_path).is_file() {
        let mut cf = File::open(&serialized_context_path).expect("cannot read saved context file");
        let mut context_as_string = String::new();
        cf.read_to_string(&mut context_as_string)
            .expect("RAND_2280042516");
        let serialized_context: SerializableContext =
            ron::de::from_str(&context_as_string).expect("Failed to deserialize context");
        //Check if file changed
        if hash != serialized_context.hash_of_original {
        } else {
            maybe_serialized_context = Some(serialized_context);
        }
    }
    if let Some(serialized_context) = maybe_serialized_context {
        ctx = Context::from_serialized_context(serialized_context, false, dumb);
    }
    //Create new Context and saved it
    else {
        let mut my_parser = antlr_parser::AntlrParser::new();
        ctx = Context::with_dump(dumb);
        if grammar_path.ends_with(".json") {
            let gf = File::open(grammar_path).expect("cannot read grammar file");
            let rules: Vec<Vec<String>> =
                serde_json::from_reader(&gf).expect("cannot parse grammar file");
            assert!(rules.len() > 0, "rule file didn_t include any rules");
            let root = "{".to_string() + &rules[0][0] + "}";
            ctx.add_rule("START", &root);
            for rule in rules {
                
                ctx.add_rule(&rule[0], &rule[1]);
            }
        } else if grammar_path.ends_with(".g4") {
            my_parser.parse_antlr_grammar(&grammar_path);
            let root = "{".to_string() + &my_parser.rules[0].0 + "}";
            ctx.add_rule("START", &root);
            for rule in my_parser.rules {
                ctx.add_rule(&rule.0, &rule.1);
            }
        } else {
            panic!("Unknown grammar type");
        }
        ctx.initialize(tree_depth, verbose);
        //Save context
        let mut cf = File::create(&serialized_context_path).expect("cannot create context file");
        let serializable_context: SerializableContext = ctx.create_serializable_context(hash);
        cf.write_all(
            ron::ser::to_string(&serializable_context)
                .expect("Serialization of Context failed!")
                .as_bytes(),
        ).expect("Writing to context file failed");
    }

    //Generate Tree
    if store {
        if Path::new("corpus").exists() {
        } else {
            fs::create_dir("corpus").expect("Could not create corpus directory");
        }
    }
    for i in 0..number_of_trees {
        let nonterm = NTermID::from(1);
        let len = ctx.get_random_len_for_nt(&nonterm);
        let generated_tree = ctx.generate_tree_from_nt(nonterm, len); //1 is the index of the "START" Node
        if verbose {
            println!("Generating tree {} from {}", i + 1, number_of_trees);
        }
        if store {
            let mut output =
                File::create(&format!("corpus/{}", i + 1)).expect("cannot create output file");
            generated_tree
                .unparse_to(&ctx, &mut output)
                .expect("Generation Failed!");
        } else {
            let stdout = io::stdout();
            let mut stdout_handle = stdout.lock();
            generated_tree
                .unparse_to(&ctx, &mut stdout_handle)
                .expect("Generation Failed!");
        }

        let mut of_tree = File::create(&"/tmp/test_tree.ron").expect("cannot create output file");
        of_tree
            .write_all(
                ron::ser::to_string(&generated_tree)
                    .expect("Serialization of Tree failed!")
                    .as_bytes(),
            )
            .expect("Writing to tree file failed");
    }
}
