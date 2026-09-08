use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;
use std::ptr;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_NULL: c_int = 5;

#[repr(C)]
struct sqlite3 {
    _private: [u8; 0],
}
#[repr(C)]
struct sqlite3_stmt {
    _private: [u8; 0],
}

#[link(name = "sqlite3")]
extern "C" {
    fn sqlite3_open(filename: *const c_char, pp_db: *mut *mut sqlite3) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        z_sql: *const c_char,
        n_byte: c_int,
        pp_stmt: *mut *mut sqlite3_stmt,
        pz_tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, i_col: c_int) -> *const u8;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, i_col: c_int) -> c_int;
    fn sqlite3_column_double(stmt: *mut sqlite3_stmt, i_col: c_int) -> f64;
    fn sqlite3_column_type(stmt: *mut sqlite3_stmt, i_col: c_int) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        cb: *const u8,
        arg: *mut u8,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(p: *mut u8);
}

pub struct Connection {
    db: *mut sqlite3,
}

unsafe impl Send for Connection {}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.db);
        }
    }
}

impl Connection {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_string_lossy();
        let c_path = CString::new(path.as_ref()).map_err(|_| "db path")?;
        let mut db = ptr::null_mut();
        let rc = unsafe { sqlite3_open(c_path.as_ptr(), &mut db) };
        if rc != SQLITE_OK {
            let msg = errmsg(db);
            unsafe { sqlite3_close(db) };
            return Err(msg);
        }
        Ok(Self { db })
    }

    pub fn query<T>(&self, sql: &str, mut f: impl FnMut(&Row) -> T) -> Result<Vec<T>, String> {
        let c_sql = CString::new(sql).map_err(|_| "sql")?;
        let mut stmt = ptr::null_mut();
        let rc =
            unsafe { sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(errmsg(self.db));
        }
        let mut rows = Vec::new();
        loop {
            let rc = unsafe { sqlite3_step(stmt) };
            if rc == SQLITE_ROW {
                rows.push(f(&Row { stmt }));
            } else if rc == SQLITE_DONE {
                break;
            } else {
                unsafe { sqlite3_finalize(stmt) };
                return Err(errmsg(self.db));
            }
        }
        unsafe { sqlite3_finalize(stmt) };
        Ok(rows)
    }

    pub fn execute(&self, sql: &str) -> Result<(), String> {
        let c_sql = CString::new(sql).map_err(|_| "sql")?;
        let mut err = ptr::null_mut();
        let rc = unsafe {
            sqlite3_exec(self.db, c_sql.as_ptr(), ptr::null(), ptr::null_mut(), &mut err)
        };
        if rc != SQLITE_OK {
            let msg = if err.is_null() {
                errmsg(self.db)
            } else {
                let s = unsafe { CStr::from_ptr(err) }
                    .to_string_lossy()
                    .into_owned();
                unsafe { sqlite3_free(err as *mut u8) };
                s
            };
            return Err(msg);
        }
        Ok(())
    }
}

pub fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub struct Row {
    stmt: *mut sqlite3_stmt,
}

impl Row {
    pub fn text(&self, i: i32) -> String {
        unsafe {
            let ptr = sqlite3_column_text(self.stmt, i);
            if ptr.is_null() {
                return String::new();
            }
            let n = sqlite3_column_bytes(self.stmt, i) as usize;
            let slice = std::slice::from_raw_parts(ptr, n);
            String::from_utf8_lossy(slice).into_owned()
        }
    }

    pub fn f64_opt(&self, i: i32) -> Option<f64> {
        unsafe {
            if sqlite3_column_type(self.stmt, i) == SQLITE_NULL {
                None
            } else {
                Some(sqlite3_column_double(self.stmt, i))
            }
        }
    }
}

fn errmsg(db: *mut sqlite3) -> String {
    if db.is_null() {
        return "sqlite error".into();
    }
    unsafe { CStr::from_ptr(sqlite3_errmsg(db)) }
        .to_string_lossy()
        .into_owned()
}
