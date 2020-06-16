use std::collections::HashSet;

use lazy_static::lazy_static;

// Reserved keywords
pub type Keyword = &'static str;
pub const ABORT: Keyword             = "ABORT";
pub const ACTION: Keyword            = "ACTION";
pub const ADD: Keyword               = "ADD";
pub const AFTER: Keyword             = "AFTER";
pub const ALL: Keyword               = "ALL";
pub const ALTER: Keyword             = "ALTER";
pub const ALWAYS: Keyword            = "ALWAYS";
pub const ANALYZE: Keyword           = "ANALYZE";
pub const AS: Keyword                = "AS";
pub const ASC: Keyword               = "ASC";
pub const ATTACH: Keyword            = "ATTACH";
pub const AUTOINCREMENT: Keyword     = "AUTOINCREMENT";
pub const BEFORE: Keyword            = "BEFORE";
pub const BEGIN: Keyword             = "BEGIN";
pub const BY: Keyword                = "BY";
pub const CASCADE: Keyword           = "CASCADE";
pub const CASE: Keyword              = "CASE";
pub const CAST: Keyword              = "CAST";
pub const CHECK: Keyword             = "CHECK";
pub const COLLATE: Keyword           = "COLLATE";
pub const COLUMN: Keyword            = "COLUMN";
pub const COMMIT: Keyword            = "COMMIT";
pub const CONFLICT: Keyword          = "CONFLICT";
pub const CONSTRAINT: Keyword        = "CONSTRAINT";
pub const CREATE: Keyword            = "CREATE";
pub const CROSS: Keyword             = "CROSS";
pub const CURRENT: Keyword           = "CURRENT";
pub const CURRENT_DATE: Keyword      = "CURRENT_DATE";
pub const CURRENT_TIME: Keyword      = "CURRENT_TIME";
pub const CURRENT_TIMESTAMP: Keyword = "CURRENT_TIMESTAMP";
pub const DATABASE: Keyword          = "DATABASE";
pub const DEFAULT: Keyword           = "DEFAULT";
pub const DEFERRABLE: Keyword        = "DEFERRABLE";
pub const DEFERRED: Keyword          = "DEFERRED";
pub const DELETE: Keyword            = "DELETE";
pub const DESC: Keyword              = "DESC";
pub const DETACH: Keyword            = "DETACH";
pub const DISTINCT: Keyword          = "DISTINCT";
pub const DO: Keyword                = "DO";
pub const DROP: Keyword              = "DROP";
pub const EACH: Keyword              = "EACH";
pub const ELSE: Keyword              = "ELSE";
pub const END: Keyword               = "END";
pub const ESCAPE: Keyword            = "ESCAPE";
pub const EXCEPT: Keyword            = "EXCEPT";
pub const EXCLUDE: Keyword           = "EXCLUDE";
pub const EXCLUSIVE: Keyword         = "EXCLUSIVE";
pub const EXISTS: Keyword            = "EXISTS";
pub const EXPLAIN: Keyword           = "EXPLAIN";
pub const FAIL: Keyword              = "FAIL";
pub const FILTER: Keyword            = "FILTER";
pub const FIRST: Keyword             = "FIRST";
pub const FOLLOWING: Keyword         = "FOLLOWING";
pub const FOR: Keyword               = "FOR";
pub const FOREIGN: Keyword           = "FOREIGN";
pub const FROM: Keyword              = "FROM";
pub const FULL: Keyword              = "FULL";
pub const GENERATED: Keyword         = "GENERATED";
pub const GROUP: Keyword             = "GROUP";
pub const GROUPS: Keyword            = "GROUPS";
pub const HAVING: Keyword            = "HAVING";
pub const IF: Keyword                = "IF";
pub const IGNORE: Keyword            = "IGNORE";
pub const IMMEDIATE: Keyword         = "IMMEDIATE";
pub const INDEX: Keyword             = "INDEX";
pub const INDEXED: Keyword           = "INDEXED";
pub const INITIALLY: Keyword         = "INITIALLY";
pub const INNER: Keyword             = "INNER";
pub const INSERT: Keyword            = "INSERT";
pub const INSTEAD: Keyword           = "INSTEAD";
pub const INTERSECT: Keyword         = "INTERSECT";
pub const INTO: Keyword              = "INTO";
pub const JOIN: Keyword              = "JOIN";
pub const KEY: Keyword               = "KEY";
pub const LAST: Keyword              = "LAST";
pub const LEFT: Keyword              = "LEFT";
pub const LIMIT: Keyword             = "LIMIT";
pub const MATCH: Keyword             = "MATCH";
pub const NATURAL: Keyword           = "NATURAL";
pub const NO: Keyword                = "NO";
pub const NOTHING: Keyword           = "NOTHING";
pub const NULLS: Keyword             = "NULLS";
pub const OF: Keyword                = "OF";
pub const OFFSET: Keyword            = "OFFSET";
pub const ON: Keyword                = "ON";
pub const ORDER: Keyword             = "ORDER";
pub const OTHERS: Keyword            = "OTHERS";
pub const OUTER: Keyword             = "OUTER";
pub const OVER: Keyword              = "OVER";
pub const PARTITION: Keyword         = "PARTITION";
pub const PLAN: Keyword              = "PLAN";
pub const PRAGMA: Keyword            = "PRAGMA";
pub const PRECEDING: Keyword         = "PRECEDING";
pub const PRIMARY: Keyword           = "PRIMARY";
pub const QUERY: Keyword             = "QUERY";
pub const RAISE: Keyword             = "RAISE";
pub const RANGE: Keyword             = "RANGE";
pub const RECURSIVE: Keyword         = "RECURSIVE";
pub const REFERENCES: Keyword        = "REFERENCES";
pub const REGEXP: Keyword            = "REGEXP";
pub const REINDEX: Keyword           = "REINDEX";
pub const RELEASE: Keyword           = "RELEASE";
pub const RENAME: Keyword            = "RENAME";
pub const REPLACE: Keyword           = "REPLACE";
pub const RESTRICT: Keyword          = "RESTRICT";
pub const RIGHT: Keyword             = "RIGHT";
pub const ROLLBACK: Keyword          = "ROLLBACK";
pub const ROW: Keyword               = "ROW";
pub const ROWS: Keyword              = "ROWS";
pub const SAVEPOINT: Keyword         = "SAVEPOINT";
pub const SELECT: Keyword            = "SELECT";
pub const SET: Keyword               = "SET";
pub const TABLE: Keyword             = "TABLE";
pub const TEMP: Keyword              = "TEMP";
pub const TEMPORARY: Keyword         = "TEMPORARY";
pub const THEN: Keyword              = "THEN";
pub const TIES: Keyword              = "TIES";
pub const TO: Keyword                = "TO";
pub const TRANSACTION: Keyword       = "TRANSACTION";
pub const TRIGGER: Keyword           = "TRIGGER";
pub const UNBOUNDED: Keyword         = "UNBOUNDED";
pub const UNION: Keyword             = "UNION";
pub const UPDATE: Keyword            = "UPDATE";
pub const USING: Keyword             = "USING";
pub const VACUUM: Keyword            = "VACUUM";
pub const VALUES: Keyword            = "VALUES";
pub const VIEW: Keyword              = "VIEW";
pub const VIRTUAL: Keyword           = "VIRTUAL";
pub const WHEN: Keyword              = "WHEN";
pub const WHERE: Keyword             = "WHERE";
pub const WINDOW: Keyword            = "WINDOW";
pub const WITH: Keyword              = "WITH";
pub const WITHOUT: Keyword           = "WITHOUT";
// Arithmetic Operators
pub const PLUS:     Keyword = "+";
pub const MINUS:    Keyword = "-";
pub const ASTERISK: Keyword = "*";
pub const SLASH:    Keyword = "/";
pub const PERCENT:  Keyword = "%";
// Comparison Operators
pub const EQ:  Keyword = "=";
pub const EQ2: Keyword = "==";
pub const NE:  Keyword = "!=";
pub const NE2: Keyword = "<>";
pub const LT:  Keyword = "<";
pub const GT:  Keyword = ">";
pub const LE:  Keyword = "<=";
pub const GE:  Keyword = ">=";
pub const NLT: Keyword = "!<";
pub const NGT: Keyword = ">";
// Logical Operators
pub const AND:     Keyword = "AND";
pub const BETWEEN: Keyword = "BETWEEN";
pub const EXISIS:  Keyword = "EXISIS";
pub const IN:      Keyword = "IN";
pub const NOT:     Keyword = "NOT";
pub const LIKE:    Keyword = "LIKE";
pub const GLOB:    Keyword = "GLOB";
pub const OR:      Keyword = "OR";
pub const IS:      Keyword = "IS";
pub const NULL:    Keyword = "NULL";
pub const CONCAT:  Keyword = "||";
pub const UNIQUE:  Keyword = "UNIQUE";
// Bitwise Operators
pub const BINAND:  Keyword = "&";
pub const BINOR:   Keyword = "|";
pub const BINFLIP: Keyword = "~";
pub const BINLS:   Keyword = "<<";
pub const BINRS:   Keyword = ">>";
// Delimiter
pub const SEMICOLON: Keyword = ";";
pub const COMMA:     Keyword = ",";
//pub const LPAREN:    Keyword = "(";
//pub const RPAREN:    Keyword = ")";

lazy_static! {
    static ref RESERVED_WORDS: HashSet<String> = {
        let mut hs = HashSet::new();
        hs.insert(SELECT.to_string());
        hs.insert(FROM.to_string());
        hs.insert(WHERE.to_string());
        // Arithmetic Operators
        hs.insert(PLUS.to_string());
        hs.insert(MINUS.to_string());
        hs.insert(ASTERISK.to_string());
        hs.insert(SLASH.to_string());
        hs.insert(PERCENT.to_string());
        // Comparison Operators
        hs.insert(EQ.to_string());
        hs.insert(EQ2.to_string());
        hs.insert(NE.to_string());
        hs.insert(NE2.to_string());
        hs.insert(LT.to_string());
        hs.insert(GT.to_string());
        hs.insert(LE.to_string());
        hs.insert(GE.to_string());
        hs.insert(NLT.to_string());
        hs.insert(NGT.to_string());
        // Logical Operators
        hs.insert(AND.to_string());
        hs.insert(BETWEEN.to_string());
        hs.insert(EXISIS.to_string());
        hs.insert(IN.to_string());
        hs.insert(NOT.to_string());
        hs.insert(LIKE.to_string());
        hs.insert(GLOB.to_string());
        hs.insert(OR.to_string());
        hs.insert(IS.to_string());
        hs.insert(NULL.to_string());
        hs.insert(CONCAT.to_string());
        hs.insert(UNIQUE.to_string());
        // Bitwise Operators
        hs.insert(BINAND.to_string());
        hs.insert(BINOR.to_string());
        hs.insert(BINFLIP.to_string());
        hs.insert(BINLS.to_string());
        hs.insert(BINRS.to_string());
        // Delimiter
        hs.insert(SEMICOLON.to_string());
        hs.insert(COMMA.to_string());
        //hs.insert(LPAREN.to_string());
        //hs.insert(RPAREN.to_string());

        hs
    };
}

pub fn is_keyword(token: &str) -> bool {
    RESERVED_WORDS.contains(&token.to_ascii_uppercase())
}

