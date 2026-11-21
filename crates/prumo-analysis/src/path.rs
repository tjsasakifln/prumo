//! PRU-025: topological load paths through components to explicit supports.
//! This enforces conservation of forces AND moments without inventing stiffness,
//! relative tributary shares or an FEM result. Every transfer share is supplied
//! and reviewed, never guessed from apparent proximity.
use crate::{AnalysisError, ensure, loads::{LoadLedger,Contribution}};
use serde::{Deserialize,Serialize};
use std::collections::{BTreeMap,BTreeSet,VecDeque};
use uuid::Uuid;

const MAX_NODES:usize=2_000;
const MAX_EDGES:usize=10_000;
fn add(a:&mut [f64;3],b:[f64;3]){
    for i in 0..3 {a[i]+=b[i];}
}
fn scale(a:[f64;3],s:f64)->[f64;3] {a.map(|x|x*s)}
fn subtract(a:[f64;3],b:[f64;3])->[f64;3] {
    std::array::from_fn(|i|a[i]-b[i])
}
fn cross(a:[f64;3],b:[f64;3])->[f64;3] {
    [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id:Uuid,
    pub position_m:[f64;3],
    pub is_support:bool,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    pub from_id:Uuid,
    pub to_id:Uuid,
    /// Explicit fraction of both resultant force and couple.
    pub share:f64,
    pub assumption:String,
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct PathModel {
    pub nodes:Vec<Node>,
    pub transfers:Vec<Transfer>,
}
#[derive(Clone,Debug,Serialize)]
pub struct SupportResult {
    pub support_id:Uuid,
    pub required_force_reaction_n:[f64;3],
    pub required_couple_reaction_nm:[f64;3],
}
#[derive(Clone,Debug,Serialize)]
pub struct TransferResult {
    pub transferred_force_n:[f64;3],
    pub transferred_couple_nm:[f64;3],
    pub from_id:Uuid,
    pub to_id:Uuid,
}
#[derive(Clone,Debug,Serialize)]
pub struct Result {
    pub supports:Vec<SupportResult>,
    pub edges:Vec<TransferResult>,
    pub external_force_n:[f64;3],
    pub external_moment_nm:[f64;3],
    pub reaction_force_n:[f64;3],
    pub reaction_moment_nm:[f64;3],
    pub global_force_imbalance_n:[f64;3],
    pub global_moment_imbalance_nm:[f64;3],
    pub assumptions:String,
    pub engineering_release:bool,
}
pub fn audit(graph:&PathModel,ledger:&LoadLedger)->std::result::Result<Result,AnalysisError>{
    ensure(!graph.nodes.is_empty() && graph.nodes.len()<=MAX_NODES &&
        graph.transfers.len()<=MAX_EDGES,
        "Rede de transferência vazia ou acima dos limites.")?;
    let mut positions=BTreeMap::new();
    for node in &graph.nodes {
        ensure(!node.id.is_nil() && positions.insert(node.id, node).is_none() &&
            node.position_m.iter().all(|v|v.is_finite()&&v.abs()<=1e8),
            "Nó de transferência inválido ou duplicado.")?;
    }
    let mut outgoing:BTreeMap<Uuid,Vec<&Transfer>>=BTreeMap::new();
    let mut indegree:BTreeMap<Uuid,usize>=positions.keys().map(|id|(*id,0)).collect();
    let mut pairs=BTreeSet::new();
    for edge in &graph.transfers {
        ensure(edge.from_id!=edge.to_id &&
            positions.contains_key(&edge.from_id)&&positions.contains_key(&edge.to_id)
            && !positions[&edge.from_id].is_support
            && edge.share.is_finite() && edge.share>0.0 && edge.share<=1.0
            && !edge.assumption.trim().is_empty()
            && pairs.insert((edge.from_id,edge.to_id)),
            "Caminho inválido, repetido ou saindo de apoio.")?;
        outgoing.entry(edge.from_id).or_default().push(edge);
        *indegree.get_mut(&edge.to_id).expect("node exists")+=1;
    }
    for (&id,edges) in &outgoing {
        let shares=edges.iter().map(|edge|edge.share).sum::<f64>();
        ensure((shares-1.0).abs()<=1e-12,
            &format!("O nó {id} não distribui exatamente 100% da resultante."))?;
    }
    // Kahn topological sort: a cycle cannot be a load path to a support.
    let mut queue:VecDeque<Uuid>=indegree.iter()
        .filter_map(|(&id,&count)|(count==0).then_some(id)).collect();
    let mut order=Vec::new();
    while let Some(id)=queue.pop_front(){
        order.push(id);
        for edge in outgoing.get(&id).into_iter().flatten() {
            let n=indegree.get_mut(&edge.to_id).expect("node exists");
            *n-=1;
            if *n==0 {queue.push_back(edge.to_id);}
        }
    }
    ensure(order.len()==graph.nodes.len(),
        "Ciclo de transferências: carga não pode circular indefinidamente.")?;
    let mut force:BTreeMap<Uuid,[f64;3]>=positions.keys().map(|id|(*id,[0.0;3])).collect();
    let mut moment:BTreeMap<Uuid,[f64;3]>=positions.keys().map(|id|(*id,[0.0;3])).collect();
    let mut loaded=BTreeSet::new();
    for Contribution {node_id,location_m,force_n,couple_nm,..} in &ledger.contributions {
        let node=positions.get(node_id)
            .ok_or_else(||AnalysisError::Invalid(format!(
                "Carga associada a nó de transferência inexistente: {node_id}")))?;
        ensure(force_n.iter().chain(couple_nm.iter()).all(|x|x.is_finite()),
            "Vetor de carga inválido.")?;
        loaded.insert(*node_id);
        add(force.get_mut(node_id).expect("node"),*force_n);
        let arm=subtract(*location_m,node.position_m);
        let mut m=cross(arm,*force_n);add(&mut m,*couple_nm);
        add(moment.get_mut(node_id).expect("node"),m);
    }
    ensure(!loaded.is_empty(),"Não há carga para transferir.")?;
    let mut edges=Vec::new();
    for id in order {
        let f=force[&id];
        let m=moment[&id];
        if let Some(next)=outgoing.get(&id) {
            for edge in next {
                let pf=scale(f,edge.share);
                let location_offset=subtract(positions[&id].position_m,
                    positions[&edge.to_id].position_m);
                let mut pm=scale(m,edge.share);
                add(&mut pm,cross(location_offset,pf));
                add(force.get_mut(&edge.to_id).expect("target"),pf);
                add(moment.get_mut(&edge.to_id).expect("target"),pm);
                edges.push(TransferResult{
                    transferred_force_n:pf,transferred_couple_nm:pm,
                    from_id:id,to_id:edge.to_id,
                });
            }
        } else if !positions[&id].is_support &&
            (f.iter().any(|x|x.abs()>1e-12)||m.iter().any(|x|x.abs()>1e-12))
        {
            return Err(AnalysisError::Invalid(format!(
                "Carga se perde no nó {id}: não existe caminho até apoio."
            )));
        }
    }
    let mut supports=Vec::new();
    let mut reaction_force=[0.0;3]; let mut reaction_moment=[0.0;3];
    for node in &graph.nodes {
        if !node.is_support {continue;}
        let f=scale(force[&node.id],-1.0);
        let couple=scale(moment[&node.id],-1.0);
        supports.push(SupportResult {
            support_id:node.id,
            required_force_reaction_n:f,
            required_couple_reaction_nm:couple,
        });
        add(&mut reaction_force,f);
        add(&mut reaction_moment,cross(node.position_m,f));
        add(&mut reaction_moment,couple);
    }
    let external_force=ledger.total_force_n;
    let external_moment=ledger.total_moment_nm;
    let force_imbalance=std::array::from_fn(|i|
        external_force[i]+reaction_force[i]);
    let moment_imbalance=std::array::from_fn(|i|
        external_moment[i]+reaction_moment[i]);
    let norm_force=external_force.iter().chain(reaction_force.iter())
        .map(|v|v.abs()).sum::<f64>().max(1.0);
    let norm_moment=external_moment.iter().chain(reaction_moment.iter())
        .map(|v|v.abs()).sum::<f64>().max(1.0);
    ensure(force_imbalance.iter().all(|v|v.is_finite()&&v.abs()/norm_force<=1e-10)
        &&moment_imbalance.iter().all(|v|v.is_finite()&&v.abs()/norm_moment<=1e-10),
        "Transferência não conservou forças e momentos.")?;
    Ok(Result {
        supports,edges,external_force_n:external_force,
        external_moment_nm:external_moment,reaction_force_n:reaction_force,
        reaction_moment_nm:reaction_moment,
        global_force_imbalance_n:force_imbalance,
        global_moment_imbalance_nm:moment_imbalance,
        assumptions:"Cotas e frações informadas pelo usuário. Resultantes requeridas no apoio não são reações calculadas por rigidez ou deformabilidade.".into(),
        engineering_release:false,
    })
}
