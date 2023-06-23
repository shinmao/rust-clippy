use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, Mutability, hir_id::HirId};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

use std::string::String;
use std::panic;

use super::utils::is_layout_incompatible;
use super::TRANSMUTE_STATISTICS;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    arg: &'tcx Expr<'_>,
) -> bool {
    // get the type information of src and dst types
    let src_type = get_ty(cx, from_ty);
    let dst_type = get_ty(cx, to_ty);

    // get current function name
    // cx.tcx.hir() will return Map
    // owner_id is hirid of owner where owner is the current function
    let owner_id = cx.enclosing_body.map(|body_id| cx.tcx.hir().body_owner(body_id)).unwrap();
    let cur_fn_name = cx.tcx.hir().name(owner_id).to_ident_string();
    println!("transmute caller:{}", cur_fn_name);

    // whether has size or alignment issues
    let unsound = if let Ok(from) = cx.tcx.try_normalize_erasing_regions(cx.param_env, from_ty)
        && let Ok(to) = cx.tcx.try_normalize_erasing_regions(cx.param_env, to_ty)
        && let Ok(from_layout) = cx.tcx.layout_of(cx.param_env.and(from_ty))
        && let Ok(to_layout) = cx.tcx.layout_of(cx.param_env.and(to_ty))
    {
        // if no any pointer type involved in transmute, then there are no any size or alignment issues
        if !(src_type.contains("&") || dst_type.contains("&") || src_type.contains("*") || dst_type.contains("*")) {
            String::from("safe")
        } else if !(from_layout.align.abi < to_layout.align.abi) {
            String::from("warn(align)")
        } else if !(from_layout.size < to_layout.size) {
            String::from("warn(size)")
        } else {
            String::from("safe")
        }

    } else {
        // no idea about layout, so don't lint
        String::from("no idea")
    };
    println!("transmute:({}>{}>{})", src_type, dst_type, unsound);
    span_lint(
        cx,
        TRANSMUTE_STATISTICS,
        e.span,
        &format!("Here is transmute({src_type}>{dst_type}=>{unsound})!"),
    );
    true
}

fn get_ty<'tcx>(cx: &LateContext<'tcx>, matched_ty: Ty<'tcx>) -> String {
    let matched_type = match &matched_ty.kind() {
        ty::Bool => String::from("bool"),
        ty::Char => String::from("char"),
        ty::Int(_) => String::from("int"),
        ty::Uint(_) => String::from("uint"),
        ty::Float(_) => String::from("float"),
        ty::Adt(adt_def, substs_def) => {
            //let matched_adt = String::from(adt_def.descr());
            let matched_adt = if adt_def.is_struct() {
                String::from("struct")
            } else if adt_def.is_union() {
                String::from("union")
            } else if adt_def.is_enum() {
                String::from("enum")
            } else {
                String::from("adt")
            };
            let man_drop = String::from("ManuallyDrop<");
            let man_drop_end = String::from(">");
            let adt_res = if adt_def.is_manually_drop() {
                man_drop + &matched_adt + &man_drop_end
            } else {
                matched_adt
            };
            adt_res
        },
        ty::Foreign(_) => String::from("ffi"),
        ty::Str => String::from("str"),
        ty::Array(arr_ty, _) => {
            let start = String::from("[");
            let end = String::from("]");
            let r = get_ty(cx, *arr_ty);
            start + &r + &end
        },
        ty::Slice(slice_ty) => {
            let start = String::from("&[");
            let end = String::from("]");
            let r = get_ty(cx, *slice_ty);
            start + &r + &end
        },
        ty::RawPtr(ptr_ty) => {
            let r = get_ty(cx, ptr_ty.ty);
            let m = if ptr_ty.mutbl == Mutability::Mut {
                "*mut "
            } else {
                "*const "
            };
            String::from(m) + &r
        },
        ty::Ref(_, ref_ty, mutbl) => {
            let r = get_ty(cx, *ref_ty);
            let m = if *mutbl == Mutability::Mut { "&mut " } else { "&" };
            String::from(m) + &r
        },
        ty::FnDef(_, _) => String::from("fn"),
        ty::FnPtr(_) => String::from("fnptr"),
        ty::Dynamic(_, _, _) => String::from("trait obj"),
        ty::Closure(_, _) => String::from("closure"),
        ty::Generator(_, _, _) => String::from("gen"),
        ty::GeneratorWitness(_) => String::from("gw"),
        ty::GeneratorWitnessMIR(_, _) => String::from("gwmir"),
        ty::Never => String::from("!"),
        ty::Tuple(_) => String::from("tuple"),
        ty::Alias(_, _) => String::from("alias"),
        ty::Param(param_ty) => {
            //let r = get_ty(cx, param_ty.to_ty(cx.tcx));
            //r
            String::from("param")
        },
        ty::Bound(_, _) => String::from("bnd"),
        ty::Placeholder(_) => String::from("ph"),
        ty::Infer(_) => String::from("infer"),
        ty::Error(_) => String::from("err"),
    };
    matched_type
}