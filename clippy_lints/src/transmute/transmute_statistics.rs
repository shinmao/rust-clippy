use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_middle::ty::layout::LayoutOf;

use std::string::String;
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
    // if no any pointer type involved in transmute, then there are no any size or alignment issues
    let unsound = if !(src_type.contains("&") || dst_type.contains("&") || src_type.contains("*") || dst_type.contains("*")) {
        String::from("safe")
    } else {
        let src_type_align = get_ty_align(cx, from_ty);
        let dst_type_align = get_ty_align(cx, to_ty);
        let src_type_size = get_ty_size(cx, from_ty);
        let dst_type_size = get_ty_size(cx, to_ty);
        if src_type_align < dst_type_align {
            format!("warn(align{}>{})", src_type_align, dst_type_align)
        } else if src_type_size < dst_type_size {
            format!("warn(size{}>{})", src_type_size, dst_type_size)
        } else if check_adt(&src_type) || check_adt(&dst_type) {
            String::from("warn(adt)")
        } else {
            String::from("safe")
        }
    };
    println!("transmute:({}>{}>{})", src_type, dst_type, unsound);
    span_lint(
        cx,
        TRANSMUTE_STATISTICS,
        e.span,
        &format!("Here is transmute({cur_fn_name}>{src_type}>{dst_type}=>{unsound})!"),
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

fn get_ty_align<'tcx>(cx: &LateContext<'tcx>, matched_ty: Ty<'tcx>) -> u64 {
    let unsound = match &matched_ty.kind() {
        ty::RawPtr(ptr_ty) => {
            if let Ok(ty_layout) = cx.layout_of(ptr_ty.ty) {
                ty_layout.align.abi.bytes()
            } else {
                0
            }
        },
        ty::Ref(_, ref_ty, _) => {
            if let Ok(ty_layout) = cx.layout_of(*ref_ty) {
                ty_layout.align.abi.bytes()
            } else {
                0
            }
        },
        _ => {
            if let Ok(ty_layout) = cx.layout_of(matched_ty) {
                ty_layout.align.abi.bytes()
            } else {
                0
            }
        },
    };
    unsound
}

fn get_ty_size<'tcx>(cx: &LateContext<'tcx>, matched_ty: Ty<'tcx>) -> u64 {
    let unsound = match &matched_ty.kind() {
        ty::RawPtr(ptr_ty) => {
            if let Ok(ty_layout) = cx.layout_of(ptr_ty.ty) {
                ty_layout.size.bytes()
            } else {
                0
            }
        },
        ty::Ref(_, ref_ty, _) => {
            if let Ok(ty_layout) = cx.layout_of(*ref_ty) {
                ty_layout.size.bytes()
            } else {
                0
            }
        },
        _ => {
            if let Ok(ty_layout) = cx.layout_of(matched_ty) {
                ty_layout.size.bytes()
            } else {
                0
            }
        },
    };
    unsound
}

fn check_adt(ty_str: &String) -> bool {
    if ty_str.contains("struct") || ty_str.contains("union") || 
        ty_str.contains("enum") || ty_str.contains("adt") || 
        ty_str.contains("trait obj") || ty_str.contains("tuple") {
            true
    } else {
        false
    }
}