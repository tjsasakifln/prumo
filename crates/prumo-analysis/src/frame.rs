//! PRU-022: two-dimensional frame element, Euler-Bernoulli or Timoshenko.
//! Each node has UX, UY (m) and RZ (rad). K uses corresponding generalized
//! SI stiffness blocks (N/m, N, N.m); the generic sparse kernel operates on
//! these consistent numerical coefficients without reinterpreting units.
//! Saint-Venant torsion, 3D flexure and constrained warping are out of scope.
use crate::{AnalysisError, ensure, sparse};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameNode {
    pub id:Uuid,
    pub position_m:[f64;2],
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Formulation {
    EulerBernoulli { slenderness_justification:Option<String> },
    Timoshenko { shear_modulus_pa:f64, shear_factor:f64 },
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameMember {
    pub id:Uuid,
    pub start_id:Uuid,
    pub end_id:Uuid,
    pub area_m2:f64,
    pub second_moment_m4:f64,
    pub elastic_modulus_pa:f64,
    pub formulation:Formulation,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameLoad {
    pub node_id:Uuid,
    pub force_x_n:f64,
    pub force_y_n:f64,
    pub moment_z_nm:f64,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameLineLoad {
    pub member_id:Uuid,
    /// Positive Y in member coordinates. Consistent Euler nodal load vector;
    /// Timoshenko variable distributed loading is not qualified in this trial.
    pub transverse_n_per_m:f64,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameSupport {
    pub node_id:Uuid,
    pub fixed:[bool;3],
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frame2D {
    pub nodes:Vec<FrameNode>,
    pub members:Vec<FrameMember>,
    pub nodal_loads:Vec<FrameLoad>,
    pub line_loads:Vec<FrameLineLoad>,
    pub supports:Vec<FrameSupport>,
}
#[derive(Clone,Debug,Serialize)]
pub struct MemberEndForces {
    pub member_id:Uuid,
    /// [N_start, V_start, M_start, N_end, V_end, M_end]
    pub local_end_forces_n_nm:[f64;6],
}
#[derive(Clone,Debug,Serialize)]
pub struct Frame2DResult {
    pub solution:sparse::Solution,
    pub member_forces:Vec<MemberEndForces>,
    pub node_ids:Vec<Uuid>,
    pub conventions:&'static str,
    pub engineering_release:bool,
}

fn matrix(member:&FrameMember,length:f64)->Result<[[f64;6];6],AnalysisError>{
    let ea=member.elastic_modulus_pa*member.area_m2;
    let ei=member.elastic_modulus_pa*member.second_moment_m4;
    ensure(ea.is_finite()&&ei.is_finite()&&ea>0.0&&ei>0.0,
        "Rigidez axial ou flexional inválida.")?;
    let slenderness=length/(member.second_moment_m4/member.area_m2).sqrt();
    let phi=match &member.formulation {
        Formulation::EulerBernoulli{slenderness_justification} => {
            if slenderness<10.0 {
                ensure(slenderness_justification.as_ref()
                    .is_some_and(|note| !note.trim().is_empty()),
                    "Euler-Bernoulli para barra baixa exige justificativa explícita.")?;
            }
            0.0
        },
        Formulation::Timoshenko{shear_modulus_pa,shear_factor}=>{
            ensure(shear_modulus_pa.is_finite()&&*shear_modulus_pa>0.0
                && shear_factor.is_finite()&&*shear_factor>0.0&&*shear_factor<=1.0,
                "Módulo de cisalhamento ou fator de seção inválido.")?;
            12.0*ei/(shear_factor*shear_modulus_pa*member.area_m2*length*length)
        }
    };
    ensure(phi.is_finite()&&phi>=0.0,"Parâmetro de cisalhamento inválido.")?;
    let axial=ea/length;
    let bending=12.0*ei/(length.powi(3)*(1.0+phi));
    let coupling=6.0*ei/(length*length*(1.0+phi));
    let rotation=(4.0+phi)*ei/(length*(1.0+phi));
    let opposite=(2.0-phi)*ei/(length*(1.0+phi));
    let mut k=[[0.0;6];6];
    k[0][0]=axial;k[0][3]=-axial;
    k[3][0]=-axial;k[3][3]=axial;
    let b=[1,2,4,5];
    let values=[
        [bending,coupling,-bending,coupling],
        [coupling,rotation,-coupling,opposite],
        [-bending,-coupling,bending,-coupling],
        [coupling,opposite,-coupling,rotation],
    ];
    for i in 0..4 {for j in 0..4 {k[b[i]][b[j]]=values[i][j];}}
    ensure(k.iter().flatten().all(|v|v.is_finite()&&v.abs()<=1e20),
        "Rigidez local fora do domínio do ensaio.")?;
    Ok(k)
}
fn transform(c:f64,s:f64)->[[f64;6];6]{
    let mut t=[[0.0;6];6];
    for base in [0,3] {
        t[base][base]=c;t[base][base+1]=s;
        t[base+1][base]=-s;t[base+1][base+1]=c;
        t[base+2][base+2]=1.0;
    }
    t
}
fn global_stiffness(k:&[[f64;6];6],t:&[[f64;6];6])->[[f64;6];6]{
    let mut out=[[0.0;6];6];
    for i in 0..6 { for j in 0..6 {
        out[i][j]=(0..6).map(|p|(0..6).map(|q|t[p][i]*k[p][q]*t[q][j])
            .sum::<f64>()).sum::<f64>();
    }}
    out
}
pub fn solve(model:&Frame2D,cancelled:Option<&AtomicBool>)
    ->Result<Frame2DResult,AnalysisError>
{
    ensure(!model.nodes.is_empty()&&!model.members.is_empty()
        && model.nodes.len()*3<=sparse::MAX_DOF
        && model.members.len()<=256,
        "Modelo de pórtico vazio ou acima dos limites.")?;
    let mut by_id=BTreeMap::new();
    for (index,node) in model.nodes.iter().enumerate(){
        ensure(!node.id.is_nil()&&by_id.insert(node.id,index).is_none()
            && node.position_m.iter().all(|v|v.is_finite()&&v.abs()<=1e8),
            "Nó inválido ou duplicado.")?;
    }
    let mut names=BTreeSet::new();
    let mut assembly=Vec::new();
    let mut loads=vec![0.0;model.nodes.len()*3];
    let mut local_records=Vec::new();
    for member in &model.members{
        ensure(!member.id.is_nil()&&names.insert(member.id)
            && member.start_id!=member.end_id,
            "Identidade ou extremidades de barra inválidas.")?;
        ensure(member.area_m2.is_finite()&&member.area_m2>0.0
            && member.second_moment_m4.is_finite()&&member.second_moment_m4>0.0
            && member.elastic_modulus_pa.is_finite()&&member.elastic_modulus_pa>0.0,
            "Seção ou módulo do pórtico inválido.")?;
        let a=*by_id.get(&member.start_id)
            .ok_or_else(||AnalysisError::Invalid("Nó inicial ausente".into()))?;
        let b=*by_id.get(&member.end_id)
            .ok_or_else(||AnalysisError::Invalid("Nó final ausente".into()))?;
        let dx=model.nodes[b].position_m[0]-model.nodes[a].position_m[0];
        let dy=model.nodes[b].position_m[1]-model.nodes[a].position_m[1];
        let length=dx.hypot(dy);
        ensure(length.is_finite()&&length>=1e-6,"Peça com comprimento reduzido.")?;
        let (c,s)=(dx/length,dy/length);
        let k=matrix(member,length)?;
        let t=transform(c,s);
        let global=global_stiffness(&k,&t);
        let dofs=[3*a,3*a+1,3*a+2,3*b,3*b+1,3*b+2];
        for i in 0..6 {for j in 0..6 {
            assembly.push(sparse::Term{
                row:dofs[i],column:dofs[j],stiffness_n_per_m:global[i][j],
            });
        }}
        let mut equivalent=[0.0;6];
        for line in model.line_loads.iter().filter(|load|load.member_id==member.id) {
            ensure(line.transverse_n_per_m.is_finite(),
                "Carga distribuída não finita.")?;
            ensure(matches!(member.formulation,Formulation::EulerBernoulli{..}),
                "Carga transversal Timoshenko requer vetor consistente próprio.")?;
            let q=line.transverse_n_per_m;
            equivalent[1]+=q*length/2.0;
            equivalent[2]+=q*length*length/12.0;
            equivalent[4]+=q*length/2.0;
            equivalent[5]-=q*length*length/12.0;
        }
        for i in 0..6 {
            loads[dofs[i]]+=(0..6).map(|j|t[j][i]*equivalent[j]).sum::<f64>();
        }
        local_records.push((member.id,dofs,k,t,equivalent));
    }
    for line in &model.line_loads {
        ensure(names.contains(&line.member_id),"Carga linear refere barra inexistente.")?;
    }
    for force in &model.nodal_loads {
        let index=*by_id.get(&force.node_id)
            .ok_or_else(||AnalysisError::Invalid("Carga nodal refere nó ausente.".into()))?;
        for (i,v) in [force.force_x_n,force.force_y_n,force.moment_z_nm].iter().enumerate() {
            ensure(v.is_finite(),"Carga nodal não finita.")?;
            loads[3*index+i]+=v;
        }
    }
    let mut constraints=Vec::new();
    let mut fixed=BTreeSet::new();
    for support in &model.supports {
        let index=*by_id.get(&support.node_id)
            .ok_or_else(||AnalysisError::Invalid("Apoio refere nó inexistente.".into()))?;
        for i in 0..3 {
            if support.fixed[i] {
                ensure(fixed.insert(3*index+i),"Restrição repetida.")?;
                constraints.push(sparse::Prescribed{dof:3*index+i,displacement_m:0.0});
            }
        }
    }
    let solution=sparse::solve(&sparse::System {
        dimension:loads.len(),terms:assembly,forces_n:loads,prescribed:constraints,
    },cancelled)?;
    let mut member_forces=Vec::new();
    for (id,dofs,k,t,equivalent) in local_records {
        let local: [f64;6]=std::array::from_fn(|i|
            (0..6).map(|j|t[i][j]*solution.displacements_m[dofs[j]]).sum::<f64>());
        let result=std::array::from_fn(|i|
            (0..6).map(|j|k[i][j]*local[j]).sum::<f64>()-equivalent[i]);
        member_forces.push(MemberEndForces{id,local_end_forces_n_nm:result});
    }
    Ok(Frame2DResult{
        solution,member_forces,
        node_ids:model.nodes.iter().map(|x|x.id).collect(),
        conventions:"Global X/Y translations in metres, RZ in radians. Local N,V in N and M in N.m. Positive tension, local +Y shear and +RZ moments. Forces refer to element ends.",
        engineering_release:false,
    })
}
