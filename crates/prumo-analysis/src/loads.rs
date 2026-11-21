//! PRU-025: attributable load distribution and equilibrium audit.
//! Force transfer to supports requires a solved, connected structural model;
//! this module does not invent load paths, reactions or hidden stiffness.
use crate::{AnalysisError, ensure, mesh};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const MAX_LOADS:usize=10_000;

fn vector_ok(value:[f64;3])->bool {
    value.iter().all(|x|x.is_finite() && x.abs()<=1e20)
}
fn cross(a:[f64;3],b:[f64;3])->[f64;3] {
    [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
}
fn add(a:&mut [f64;3],b:[f64;3]){
    for i in 0..3 {a[i]+=b[i];}
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodalLoad {
    pub id:Uuid,
    pub origin_id:Uuid,
    pub node_id:Uuid,
    pub location_m:[f64;3],
    pub force_n:[f64;3],
    pub couple_nm:[f64;3],
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct UniformLine {
    pub id:Uuid,
    pub start_node:Uuid,
    pub end_node:Uuid,
    pub start_m:[f64;3],
    pub end_m:[f64;3],
    pub force_n_per_m:[f64;3],
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Vertex {
    pub node_id:Uuid,
    pub position_m:[f64;3],
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct UniformSurface {
    pub id:Uuid,
    pub physical_object_id:Uuid,
    pub vertices:Vec<Vertex>,
    /// Only material area triangles. Openings must have no triangles.
    pub triangles:Vec<[usize;3]>,
    pub force_n_per_m2:[f64;3],
    pub own_weight:bool,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedLoads {
    pub points:Vec<NodalLoad>,
    pub lines:Vec<UniformLine>,
    pub surfaces:Vec<UniformSurface>,
}
#[derive(Clone,Debug,Serialize)]
pub struct Contribution {
    pub source_id:Uuid,
    pub node_id:Uuid,
    pub location_m:[f64;3],
    pub force_n:[f64;3],
    pub couple_nm:[f64;3],
}
#[derive(Clone,Debug,Serialize)]
pub struct LoadLedger {
    pub contributions:Vec<Contribution>,
    pub total_force_n:[f64;3],
    pub total_moment_nm:[f64;3],
    pub number_of_sources:usize,
    pub analysis_approved:bool,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    pub node_id:Uuid,
    pub location_m:[f64;3],
    pub force_n:[f64;3],
    pub couple_nm:[f64;3],
}
#[derive(Clone,Debug,Serialize)]
pub struct Equilibrium {
    pub force_imbalance_n:[f64;3],
    pub moment_imbalance_nm:[f64;3],
    pub force_relative_error:f64,
    pub moment_relative_error:f64,
    pub within_tolerance:bool,
    pub engineering_release:bool,
}

pub fn ledger(input:&AppliedLoads)->Result<LoadLedger,AnalysisError>{
    let count=input.points.len()+input.lines.len()+input.surfaces.len();
    if count>MAX_LOADS {return Err(AnalysisError::ResourceLimit("fontes de carga excedidas".into()));}
    let mut sources=BTreeSet::new();
    let mut selfweight=BTreeSet::new();
    let mut contributions=Vec::new();
    for item in &input.points {
        ensure(!item.id.is_nil() && !item.node_id.is_nil() && !item.origin_id.is_nil()
            && sources.insert(item.id) && vector_ok(item.location_m)
            && vector_ok(item.force_n) && vector_ok(item.couple_nm),
            "Carga concentrada com ID repetido, referência nula ou vetor inválido.")?;
        contributions.push(Contribution{
            source_id:item.id,node_id:item.node_id,location_m:item.location_m,
            force_n:item.force_n,couple_nm:item.couple_nm,
        });
    }
    for line in &input.lines {
        ensure(!line.id.is_nil() && !line.start_node.is_nil() && !line.end_node.is_nil()
            && line.start_node!=line.end_node && sources.insert(line.id)
            && vector_ok(line.start_m) && vector_ok(line.end_m)
            && vector_ok(line.force_n_per_m),
            "Carga linear com identidade, nós ou geometria inválida.")?;
        let distance: [f64;3]=std::array::from_fn(|i|line.end_m[i]-line.start_m[i]);
        let length=distance.iter().map(|x|x*x).sum::<f64>().sqrt();
        ensure(length.is_finite() && length>1e-8,"Carga linear de comprimento nulo.")?;
        let end_force=line.force_n_per_m.map(|v|v*length/2.0);
        ensure(vector_ok(end_force),"Força equivalente não finita.")?;
        for (id,loc) in [(line.start_node,line.start_m),(line.end_node,line.end_m)] {
            contributions.push(Contribution {
                source_id:line.id,node_id:id,location_m:loc,
                force_n:end_force,couple_nm:[0.0;3],
            });
        }
    }
    for surface in &input.surfaces {
        ensure(!surface.id.is_nil() && !surface.physical_object_id.is_nil()
            && sources.insert(surface.id) && vector_ok(surface.force_n_per_m2),
            "Carga de área inválida ou duplicada.")?;
        if surface.own_weight {
            ensure(selfweight.insert(surface.physical_object_id),
                "Peso próprio repetido para o mesmo objeto físico.")?;
        }
        let inspector=mesh::inspect(&mesh::Mesh{
            points_m:surface.vertices.iter().map(|v|v.position_m).collect(),
            triangles:surface.triangles.clone(),thickness_m:1.0,
        })?;
        ensure(surface.vertices.iter().all(|v|!v.node_id.is_nil()) &&
            inspector.elements.len()==surface.triangles.len(),
            "Nós ou triângulos de área inválidos.")?;
        let mut by_node:BTreeMap<Uuid,([f64;3],[f64;3])>=BTreeMap::new();
        for v in &surface.vertices {
            if let Some((previous,_))=by_node.get(&v.node_id) {
                ensure(*previous==v.position_m,"Nó tem coordenadas contraditórias.")?;
            } else { by_node.insert(v.node_id,(v.position_m,[0.0;3])); }
        }
        for (triangle,quality) in surface.triangles.iter().zip(&inspector.elements) {
            let portion=surface.force_n_per_m2.map(|value|
                value*quality.area_m2/3.0);
            ensure(vector_ok(portion),"Resultante de superfície fora do domínio.")?;
            for &corner in triangle {
                let vertex=&surface.vertices[corner];
                let record=by_node.get_mut(&vertex.node_id).expect("vertex");
                add(&mut record.1,portion);
            }
        }
        for (node_id,(location_m,force_n)) in by_node {
            contributions.push(Contribution {
                source_id:surface.id,node_id,location_m,
                force_n,couple_nm:[0.0;3],
            });
        }
    }
    let mut force=[0.0;3];let mut moment=[0.0;3];
    for component in &contributions {
        add(&mut force,component.force_n);
        add(&mut moment,cross(component.location_m,component.force_n));
        add(&mut moment,component.couple_nm);
    }
    ensure(vector_ok(force)&&vector_ok(moment),
        "Balanço de forças ou momentos fora do domínio numérico.")?;
    Ok(LoadLedger {
        contributions,total_force_n:force,total_moment_nm:moment,
        number_of_sources:sources.len(),analysis_approved:false,
    })
}
/// Checks global force and moment closure, without assuming reactions are real.
pub fn check_equilibrium(ledger:&LoadLedger,reactions:&[Reaction],tolerance:f64)
    ->Result<Equilibrium,AnalysisError>
{
    ensure(tolerance.is_finite()&&tolerance>0.0&&tolerance<=1e-3,
        "Tolerância de equilíbrio fora do domínio.")?;
    ensure(!reactions.is_empty(),"São necessárias reações calculadas e verificadas.")?;
    let mut force=ledger.total_force_n;
    let mut moment=ledger.total_moment_nm;
    let mut magnitude_force=ledger.total_force_n.iter().map(|x|x.abs()).sum::<f64>();
    let mut magnitude_moment=ledger.total_moment_nm.iter().map(|x|x.abs()).sum::<f64>();
    for r in reactions {
        ensure(!r.node_id.is_nil()&&vector_ok(r.location_m)
            &&vector_ok(r.force_n)&&vector_ok(r.couple_nm),
            "Reação com identidade, coordenadas ou vetor inválido.")?;
        add(&mut force,r.force_n);
        let contribution=cross(r.location_m,r.force_n);
        add(&mut moment,contribution);
        add(&mut moment,r.couple_nm);
        magnitude_force+=r.force_n.iter().map(|x|x.abs()).sum::<f64>();
        magnitude_moment+=contribution.iter().chain(r.couple_nm.iter()).map(|x|x.abs()).sum::<f64>();
    }
    let max_force=force.iter().map(|x|x.abs()).fold(0.0_f64,f64::max);
    let max_moment=moment.iter().map(|x|x.abs()).fold(0.0_f64,f64::max);
    let fr=max_force/magnitude_force.max(1.0);
    let mr=max_moment/magnitude_moment.max(1.0);
    ensure(fr.is_finite()&&mr.is_finite(),"Balanço não finito.")?;
    Ok(Equilibrium {
        force_imbalance_n:force,moment_imbalance_nm:moment,
        force_relative_error:fr,moment_relative_error:mr,
        within_tolerance:fr<=tolerance&&mr<=tolerance,
        engineering_release:false,
    })
}
