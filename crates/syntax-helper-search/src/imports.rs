use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::convert::Infallible;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, OptionalExtension, Statement, params};
use strsim::levenshtein;
use syntax_helper_language as language;
pub use syntax_helper_model as model;
use thiserror::Error;

pub const INDEX_SCHEMA_VERSION: u32 = 15;
const TYPE_REFERENCE_RELATION_WEIGHT: i64 = 12;

type TypeTemplateRow = (
    String,
    Vec<String>,
    Option<model::PlatformTypeTemplateKey>,
    Option<String>,
);
