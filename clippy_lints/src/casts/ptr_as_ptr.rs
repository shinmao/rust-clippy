use clippy_utils::diagnostics::span_lint;
use clippy_utils::msrvs::{self, Msrv};
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, Mutability, hir_id::HirId};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, TypeAndMut, Ty};
use rustc_span::symbol::sym;

use super::PTR_AS_PTR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, msrv: &Msrv) {
    if !msrv.meets(msrvs::POINTER_CAST) {
        return;
    }

    if_chain! {
        if let ExprKind::Cast(cast_expr, cast_to_hir_ty) = expr.kind;
        //let (cast_from_hirid, cast_to_hirid) = (cast_expr.hir_id, expr.hir_id);
        let (cast_from, cast_to) = (cx.typeck_results().expr_ty(cast_expr), cx.typeck_results().expr_ty(expr));
        if let ty::RawPtr(TypeAndMut { ty: from_pointer_ty, mutbl: from_mutbl }) = cast_from.kind();
        if let ty::RawPtr(TypeAndMut { ty: to_pointee_ty, mutbl: to_mutbl }) = cast_to.kind();
        // The `U` in `pointer::cast` have to be `Sized`
        // as explained here: https://github.com/rust-lang/rust/issues/60602.
        if to_pointee_ty.is_sized(cx.tcx, cx.param_env);
        then {
            let from_ty = get_ty(cx, *from_pointer_ty);
            let to_ty = get_ty(cx, *to_pointee_ty);

            // get current function name
            // cx.tcx.hir() will return Map
            // owner_id is hirid of owner where owner is the current function
            let owner_id = cx.enclosing_body.map(|body_id| cx.tcx.hir().body_owner(body_id)).unwrap();
            let cur_fn_name = cx.tcx.hir().name(owner_id).to_ident_string();
            println!("casting caller:{}", cur_fn_name);

            let align_res = if let Ok(from) = cx.tcx.try_normalize_erasing_regions(cx.param_env, cast_from)
                && let Ok(to) = cx.tcx.try_normalize_erasing_regions(cx.param_env, cast_to)
                && let Ok(from_layout) = cx.tcx.layout_of(cx.param_env.and(cast_from))
                && let Ok(to_layout) = cx.tcx.layout_of(cx.param_env.and(cast_to))
            {
                if from_layout.align.abi.bytes() < to_layout.align.abi.bytes() {
                    format!("warn(align{}>{})", from_layout.align.abi.bytes(), to_layout.align.abi.bytes())
                } else if from_layout.size.bytes() < to_layout.size.bytes() {
                    format!("warn(size{}>{})", from_layout.size.bytes(), to_layout.size.bytes())
                } else if check_adt(&from_ty) || check_adt(&to_ty) {
                    String::from("warn(adt)")
                }  else {
                    String::from("safe")
                }

            } else {
                // no idea about layout, so don't lint
                String::from("no idea")
            };

            match (from_mutbl, to_mutbl) {
                (Mutability::Not, Mutability::Not) => {
                    span_lint(
                        cx,
                        PTR_AS_PTR,
                        expr.span,
                        &format!("Here is cast({cur_fn_name}>{from_ty}>{to_ty}=>safe>{align_res})!"),
                    );
                },
                (Mutability::Mut, Mutability::Mut) => {
                    span_lint(
                        cx,
                        PTR_AS_PTR,
                        expr.span,
                        &format!("Here is cast({cur_fn_name}>{from_ty}>{to_ty}=>mut_to_mut>{align_res})!"),
                    );
                },
                (Mutability::Not, Mutability::Mut) => {
                    span_lint(
                        cx,
                        PTR_AS_PTR,
                        expr.span,
                        &format!("Here is cast({cur_fn_name}>{from_ty}>{to_ty}=>immut_to_mut>{align_res})!"),
                    );
                },
                _ => println!("nothing"),
            }
        }
    }
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

fn check_adt(ty_str: &String) -> bool {
    if (ty_str.contains("struct") || ty_str.contains("union") || 
        ty_str.contains("enum") || ty_str.contains("adt") || 
        ty_str.contains("trait obj") || ty_str.contains("tuple")) {
            true
    } else {
        false
    }
}

fn has_c_repr_attr(cx: &LateContext<'_>, hir_id: HirId, ty_str: &String) -> bool {
    cx.tcx.hir().attrs(hir_id).iter().any(|attr| {
        if attr.has_name(sym::repr) {
            if let Some(items) = attr.meta_item_list() {
                for item in items {
                    if item.is_word() 
                        && matches!(item.name_or_empty(), sym::C) 
                        && (ty_str.contains("struct") || ty_str.contains("union") || 
                            ty_str.contains("enum") || ty_str.contains("adt") || 
                            ty_str.contains("trait obj") || ty_str.contains("tuple"))
                    {
                        return true;
                    }
                }
            }
        }
        false
    })
}