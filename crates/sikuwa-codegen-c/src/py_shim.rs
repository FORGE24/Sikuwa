//! Emit Python extension module shim (`_pywrap.c`).

use std::fmt::Write;

use sikuwa_pystat::{FuncStat, PystatReport};

use crate::emit::{module_c_name, skw_c_symbol};

pub fn emit_pywrap_c(module_name: &str, report: &PystatReport) -> String {
    let mut out = String::new();
    let mod_c = module_c_name(module_name);
    let _ = writeln!(out, "/* generated python embed shim — requires Python headers */");
    let _ = writeln!(out, "#define PY_SSIZE_T_CLEAN");
    let _ = writeln!(out, "#include <Python.h>");
    let _ = writeln!(out, "#include \"{module_name}.h\"");
    let _ = writeln!(out);

    let mut methods = Vec::new();
    for f in &report.module.functions {
        if !f.static_eligible || f.return_ty != sikuwa_pystat::PhysicalType::Int64 {
            continue;
        }
        if f.params.len() != 2
            || f.params.iter().any(|p| p.ty != sikuwa_pystat::PhysicalType::Int64)
        {
            continue;
        }
        let py_fn = py_method_name(&f.symbol.0);
        let c_fn = skw_c_symbol(&f.symbol.0);
        let _ = writeln!(
            out,
            "static PyObject *py_{py_fn}(PyObject *self, PyObject *args) {{\n    (void)self;\n    long a, b;\n    if (!PyArg_ParseTuple(args, \"ll\", &a, &b)) return NULL;\n    return PyLong_FromLongLong({c_fn}(a, b));\n}}\n"
        );
        methods.push((py_fn, f.symbol.0.clone()));
    }

    let _ = writeln!(out, "static PyMethodDef skw_{mod_c}_methods[] = {{");
    for (py_fn, doc_sym) in &methods {
        let _ = writeln!(
            out,
            "    {{\"{py_fn}\", (PyCFunction)py_{py_fn}, METH_VARARGS, \"{doc_sym}\"}},"
        );
    }
    let _ = writeln!(out, "    {{NULL, NULL, 0, NULL}}");
    let _ = writeln!(out, "}};");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "static struct PyModuleDef skw_{mod_c}_module = {{\n    PyModuleDef_HEAD_INIT,\n    \"{module_name}\",\n    NULL,\n    -1,\n    skw_{mod_c}_methods\n}};\n"
    );
    let _ = writeln!(
        out,
        "PyMODINIT_FUNC PyInit_{module_name}(void) {{\n    return PyModule_Create(&skw_{mod_c}_module);\n}}\n"
    );
    out
}

fn py_method_name(symbol: &str) -> String {
    symbol.rsplit('.').next().unwrap_or("func").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sikuwa_pir::{lower_source, sample_add_module};
    use sikuwa_pystat::analyze_module;

    #[test]
    fn pywrap_has_init() {
        let pir = sample_add_module();
        let report = analyze_module(&pir);
        let c = emit_pywrap_c("sample", &report);
        assert!(c.contains("PyInit_sample"));
        assert!(c.contains("py_add"));
    }
}
