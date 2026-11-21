use prumo_analysis::{AnalysisError, constraints, loads, mesh, sparse, truss};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

fn term(row:usize,column:usize,k:f64)->sparse::Term {
    sparse::Term {row,column,stiffness_n_per_m:k}
}
#[test]
fn sparse_spd_analytical_inverse_and_order_independence() {
    let original=sparse::System {
        dimension:2, terms:vec![term(0,0,4.0),term(0,1,1.0),term(1,0,1.0),term(1,1,3.0)],
        forces_n:vec![1.0,2.0],prescribed:vec![],
    };
    let result=sparse::solve(&original,None).unwrap();
    assert!((result.displacements_m[0]-1.0/11.0).abs()<1e-12);
    assert!((result.displacements_m[1]-7.0/11.0).abs()<1e-12);
    assert!(result.free_relative_residual < sparse::RESIDUAL_LIMIT);
    assert!(!result.engineering_release);
    let permuted=sparse::System {
        dimension:2,terms:vec![term(1,1,4.0),term(1,0,1.0),term(0,1,1.0),term(0,0,3.0)],
        forces_n:vec![2.0,1.0],prescribed:vec![],
    };
    let reordered=sparse::solve(&permuted,None).unwrap();
    assert!((reordered.displacements_m[0]-result.displacements_m[1]).abs()<1e-12);
    assert!((reordered.displacements_m[1]-result.displacements_m[0]).abs()<1e-12);
}
#[test]
fn sparse_reactions_mechanism_conditioning_and_cancel_are_explicit() {
    let fixed=sparse::System {
        dimension:2,terms:vec![term(0,0,1000.0),term(0,1,-1000.0),
            term(1,0,-1000.0),term(1,1,1000.0)],
        forces_n:vec![0.0,10.0],
        prescribed:vec![sparse::Prescribed{dof:0,displacement_m:0.0}],
    };
    let r=sparse::solve(&fixed,None).unwrap();
    assert!((r.displacements_m[1]-0.01).abs()<1e-12);
    assert!((r.reactions_n[0]+10.0).abs()<1e-12);
    assert!(matches!(sparse::solve(&sparse::System{prescribed:vec![],..fixed.clone()},None),
        Err(AnalysisError::Mechanism(_))));
    let ill=sparse::System{
        dimension:2,terms:vec![term(0,0,1e12),term(1,1,1e-3)],
        forces_n:vec![1.0,1.0],prescribed:vec![],
    };
    assert!(matches!(sparse::solve(&ill,None),Err(AnalysisError::Conditioning(_))));
    let flag=AtomicBool::new(true);
    assert!(matches!(sparse::solve(&fixed,Some(&flag)),Err(AnalysisError::Cancelled)));
}
#[test]
fn truss_3d_axial_and_no_fictitious_transverse_stiffness() {
    let a=Uuid::new_v4();let b=Uuid::new_v4();
    let mut model=truss::Truss {
        nodes:vec![truss::Node{id:a,position_m:[0.0,0.0,0.0]},
            truss::Node{id:b,position_m:[2.0,0.0,0.0]}],
        bars:vec![truss::AxialBar{id:Uuid::new_v4(),start_id:a,end_id:b,
            area_m2:0.01,elastic_modulus_pa:200e9}],
        forces:vec![truss::NodalForce{node_id:b,force_n:[1000.0,0.0,0.0]}],
        supports:vec![truss::Support{node_id:a,fixed:[true;3]},
            truss::Support{node_id:b,fixed:[false,true,true]}],
    };
    let result=truss::solve(&model).unwrap();
    assert!((result.analysis.displacements_m[3]-1e-6).abs()<1e-12);
    assert!((result.axial[0].axial_force_n-1000.0).abs()<1e-6);
    assert!(result.force_balance_n.iter().all(|v|v.abs()<1e-6));
    assert!(!result.engineering_release);
    model.supports.pop();
    assert!(matches!(truss::solve(&model),Err(AnalysisError::Mechanism(_))));
}
#[test]
fn mesh_area_refinement_and_orientation_are_checked() {
    let triangle=mesh::Mesh{
        points_m:vec![[0.0,0.0,0.0],[2.0,0.0,0.0],[0.0,2.0,0.0]],
        triangles:vec![[0,1,2]],thickness_m:0.15,
    };
    let inspected=mesh::inspect(&triangle).unwrap();
    assert!((inspected.total_area_m2-2.0).abs()<1e-12);
    assert!(!inspected.analysis_available);
    let refined=mesh::refine(&triangle).unwrap();
    assert_eq!(refined.triangles.len(),4);
    assert!((mesh::inspect(&refined).unwrap().total_area_m2-2.0).abs()<1e-12);
    let flipped=mesh::Mesh{
        triangles:vec![[0,1,2],[1,0,2]],..triangle
    };
    assert!(mesh::inspect(&flipped).is_err());
}
#[test]
fn converge_only_with_multiple_refinements_and_stable_values() {
    let values=vec![
        mesh::ConvergencePoint{elements:10,value:1.0,unit:"m".into()},
        mesh::ConvergencePoint{elements:40,value:0.91,unit:"m".into()},
        mesh::ConvergencePoint{elements:160,value:0.905,unit:"m".into()},
    ];
    let result=mesh::convergence(&values,0.01).unwrap();
    assert!(result.converged);
    assert!(!result.engineering_release);
    let mut bad=values;
    bad[2].elements=30;
    assert!(mesh::convergence(&bad,0.01).is_err());
}
#[test]
fn mpc_condensation_recovers_energy_and_detects_cycles() {
    let original=sparse::System{
        dimension:3,
        terms:vec![term(0,0,2.0),term(1,1,2.0),term(2,2,2.0)],
        forces_n:vec![0.0,0.0,6.0],prescribed:vec![],
    };
    let constraint=constraints::Constraint{
        slave_dof:2,masters:vec![constraints::Master{dof:1,coefficient:1.0}],
        offset_m:0.0,justification:"diafragma de ensaio".into(),
    };
    let reduced=constraints::reduce(&original,&[constraint.clone()]).unwrap();
    let r=sparse::solve(&reduced.condensed,None).unwrap();
    let recovered=constraints::recover(&original,&reduced,&r).unwrap();
    assert!((recovered.full_displacements_m[1]-1.5).abs()<1e-12);
    assert_eq!(recovered.full_displacements_m[1],recovered.full_displacements_m[2]);
    assert!((recovered.strain_energy_j-4.5).abs()<1e-12);
    assert!(!recovered.engineering_release);
    let cyclic=vec![constraint,constraints::Constraint{
        slave_dof:1,masters:vec![constraints::Master{dof:2,coefficient:1.0}],
        offset_m:0.0,justification:"ciclo".into(),
    }];
    assert!(constraints::reduce(&original,&cyclic).is_err());
}
#[test]
fn line_and_surface_resultants_preserve_force_and_moment() {
    let a=Uuid::new_v4();let b=Uuid::new_v4();let c=Uuid::new_v4();
    let object=Uuid::new_v4();
    let input=loads::AppliedLoads{
        points:vec![],lines:vec![loads::UniformLine{
            id:Uuid::new_v4(),start_node:a,end_node:b,
            start_m:[0.0,0.0,0.0],end_m:[4.0,0.0,0.0],
            force_n_per_m:[0.0,0.0,-10.0]}],
        surfaces:vec![loads::UniformSurface{
            id:Uuid::new_v4(),physical_object_id:object,
            vertices:vec![
                loads::Vertex{node_id:a,position_m:[0.0,0.0,0.0]},
                loads::Vertex{node_id:b,position_m:[2.0,0.0,0.0]},
                loads::Vertex{node_id:c,position_m:[0.0,2.0,0.0]}],
            triangles:vec![[0,1,2]],
            force_n_per_m2:[0.0,0.0,-100.0],own_weight:true,
        }],
    };
    let ledger=loads::ledger(&input).unwrap();
    assert_eq!(ledger.number_of_sources,2);
    assert!((ledger.total_force_n[2]+240.0).abs()<1e-9);
    // Total about world origin: line +80 Nm about Y,
    // triangle = (-400/3,+400/3,0) Nm.
    assert!((ledger.total_moment_nm[0]+400.0/3.0).abs()<1e-9);
    assert!((ledger.total_moment_nm[1]-80.0-400.0/3.0).abs()<1e-9);
    let centroid=[-ledger.total_moment_nm[1]/ledger.total_force_n[2],
        ledger.total_moment_nm[0]/ledger.total_force_n[2],0.0];
    let r=loads::Reaction{
        node_id:Uuid::new_v4(),location_m:centroid,
        force_n:[0.0,0.0,240.0],couple_nm:[0.0;3],
    };
    let closure=loads::check_equilibrium(&ledger,&[r],1e-12).unwrap();
    assert!(closure.within_tolerance);
    assert!(!closure.engineering_release);
    let mut repeated=input;
    repeated.surfaces.push(loads::UniformSurface{
        id:Uuid::new_v4(),..repeated.surfaces[0].clone()
    });
    assert!(loads::ledger(&repeated).is_err());
}


#[test]
fn cantilever_frame_matches_closed_euler_and_timoshenko_references() {
    use prumo_analysis::frame;
    let a=Uuid::new_v4();let b=Uuid::new_v4();let bar=Uuid::new_v4();
    let mut model=frame::Frame2D{
        nodes:vec![
            frame::FrameNode{id:a,position_m:[0.0,0.0]},
            frame::FrameNode{id:b,position_m:[2.0,0.0]}
        ],
        members:vec![frame::FrameMember{
            id:bar,start_id:a,end_id:b,
            area_m2:0.02,second_moment_m4:8e-6,elastic_modulus_pa:200e9,
            formulation:frame::Formulation::EulerBernoulli{
                slenderness_justification:None
            }
        }],
        nodal_loads:vec![frame::FrameLoad{
            node_id:b,force_x_n:0.0,force_y_n:-1000.0,moment_z_nm:0.0
        }],
        line_loads:vec![],
        supports:vec![frame::FrameSupport{node_id:a,fixed:[true;3]}],
    };
    let euler=frame::solve(&model,None).unwrap();
    let closed=-1000.0*2.0_f64.powi(3)/(3.0*200e9*8e-6);
    assert!((euler.solution.displacements_m[4]-closed).abs()<1e-11);
    assert!((euler.solution.displacements_m[5]+0.00125).abs()<1e-11);
    assert!((euler.solution.reactions_n[1]-1000.0).abs()<1e-6);
    assert!((euler.solution.reactions_n[2]-2000.0).abs()<1e-6);
    assert!(!euler.engineering_release);
    model.members[0].formulation=frame::Formulation::Timoshenko{
        shear_modulus_pa:80e9,shear_factor:5.0/6.0,
    };
    let tim=frame::solve(&model,None).unwrap();
    let shear=-1000.0*2.0/(80e9*0.02*(5.0/6.0));
    assert!((tim.solution.displacements_m[4]-(closed+shear)).abs()<1e-10);
}
#[test]
fn simply_supported_uniform_load_has_zero_end_moments_and_equal_reactions() {
    use prumo_analysis::frame;
    let a=Uuid::new_v4();let b=Uuid::new_v4();let member=Uuid::new_v4();
    let model=frame::Frame2D{
        nodes:vec![
            frame::FrameNode{id:a,position_m:[0.0,0.0]},
            frame::FrameNode{id:b,position_m:[4.0,0.0]},
        ],
        members:vec![frame::FrameMember{
            id:member,start_id:a,end_id:b,
            area_m2:0.05,second_moment_m4:1e-4,
            elastic_modulus_pa:200e9,
            formulation:frame::Formulation::EulerBernoulli{
                slenderness_justification:None,
            },
        }],
        nodal_loads:vec![],
        line_loads:vec![frame::FrameLineLoad{
            member_id:member,transverse_n_per_m:-1000.0,
        }],
        supports:vec![
            frame::FrameSupport{node_id:a,fixed:[true,true,false]},
            frame::FrameSupport{node_id:b,fixed:[false,true,false]},
        ],
    };
    let result=frame::solve(&model,None).unwrap();
    assert!((result.solution.reactions_n[1]-2000.0).abs()<1e-6);
    assert!((result.solution.reactions_n[4]-2000.0).abs()<1e-6);
    assert!(result.member_forces[0].local_end_forces_n_nm[2].abs()<1e-5);
    assert!(result.member_forces[0].local_end_forces_n_nm[5].abs()<1e-5);
}


#[test]
fn plane_stress_membrane_passes_constant_strain_patch_reference() {
    use prumo_analysis::membrane;
    let mesh=mesh::Mesh {
        points_m:vec![[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]],
        triangles:vec![[0,1,2]], thickness_m:0.1,
    };
    let displacements=[[0.0,0.0],[0.001,0.0],[0.003,0.002]];
    let mut supports=Vec::new();
    for (vertex,values) in displacements.iter().enumerate(){
        for axis in 0..2 {
            supports.push(membrane::MembraneSupport{
                vertex,axis,displacement_m:values[axis],
            });
        }
    }
    let input=membrane::Membrane {
        mesh,material:membrane::MembraneMaterial{
            elastic_modulus_pa:200e9,poisson_ratio:0.25,
        },
        forces:vec![],supports,
    };
    let result=membrane::solve(&input,None).unwrap();
    let a=&result.elements[0];
    for (got,ref_value) in a.strain.iter().zip([0.001,0.002,0.003]) {
        assert!((*got-ref_value).abs()<1e-12);
    }
    for (got,ref_value) in a.stress_pa.iter().zip([320e6,480e6,240e6]) {
        assert!((*got-ref_value).abs()<1e-2);
    }
    assert!((result.total_strain_energy_j-50_000.0).abs()<1e-5);
    assert!(!result.engineering_release);
    // Free membrane retains its rigid-body modes; never stabilize artificially.
    let mut unsupported=input;
    unsupported.supports.clear();
    assert!(membrane::solve(&unsupported,None).is_err());
}
