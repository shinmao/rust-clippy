use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, Mutability};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

use std::error;
use std::string::String;

use super::utils::is_layout_incompatible;
use super::TRANSMUTE_STATISTICS;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    arg: &'tcx Expr<'_>,
) -> bool {
    let src_type = get_ty(cx, from_ty);
    let dst_type = get_ty(cx, to_ty);
    let unsound = if let Ok(from) = cx.tcx.try_normalize_erasing_regions(cx.param_env, from_ty)
        && let Ok(to) = cx.tcx.try_normalize_erasing_regions(cx.param_env, to_ty)
        && let Ok(from_layout) = cx.tcx.layout_of(cx.param_env.and(from_ty))
        && let Ok(to_layout) = cx.tcx.layout_of(cx.param_env.and(to_ty))
    {
        if !(from_layout.size != to_layout.size || from_layout.align.abi < to_layout.align.abi) {
            String::from("warn")
        } else {
            String::from("safe")
        }
        // let split = String::from("/");
        // let from_layout_size = from_layout.size.raw.to_string();
        // let to_layout_size = to_layout.size.raw.to_string();
        // let from_layout_align = from_layout.align.abi.pow2.to_string();
        // let to_layout_align = to_layout.align.abi.pow2.to_string();
        // from_layout_size + &split + &to_layout_size + &split + &from_layout_align + &split + &to_layout_align

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
            let matched_adt = String::from(adt_def.descr());
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
