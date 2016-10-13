use super::Loc;
use super::Element;
use super::Stroke;
use super::Feature;
use super::Point;
use super::Settings;

pub struct Optimizer {
    elements: Vec<(Loc, Vec<Element>)>,
    /// eaten elements, loc of the component
    /// and the index of the element in the compponent
    /// 0 when there is only 1 element
    eaten: Vec<(Loc, usize)>
}

impl Optimizer {
    pub fn new(elements: Vec<(Loc, Vec<Element>)>) -> Optimizer {
        Optimizer 
            { elements: elements,
              eaten :vec![]
            }
    }

    fn trace(&mut self, loc: &Loc, chain: &Vec<Element>) -> Vec<Element>{
        let neighbors = loc.neighbors();
        for neighbor in neighbors {
            let absorbs = self.absorb_reduce(&neighbor, chain);
            match absorbs {
                Some(absorbs) => {
                    let mut extended_chain = vec![];
                    extended_chain.extend(chain.clone());
                    let traced = self.trace(&neighbor, &absorbs);
                    return traced;
                },
                None => {
                    println!("nothing absorved in this neighbor: {:?}", neighbor);
                    println!("moving on...");
                    return chain.clone();
                },
            }
        }
        chain.clone()
    }

    fn absorb_reduce(&mut self, loc: &Loc, chain: &Vec<Element>) -> Option<Vec<Element>> {
        let last = chain.iter().last(); 
        match last{
            Some(last) => {
                let absorbs = self.absorb(loc, last);
                if absorbs.len() > 0 {
                    let mut all = vec![];
                    let mut eater = last.clone();
                    let mut unreduced = vec![];
                    for abs in absorbs{
                        match eater.reduce(&abs){
                            Some(reduced) => {
                                eater = reduced;
                            },
                            None => {
                                unreduced.push(abs);
                            }
                        }
                    }
                    all.push(eater);
                    all.extend(unreduced);
                    Some(all)
                }else{
                    None
                }
            },
            None => None
        }
    }

    /// recursively call until can pick element within the component
    /// mark eaten those element which are successfully picked
    fn absorb(&mut self, loc: &Loc, last_elem: &Element) -> Vec<Element> {
        let mut component_chain = vec![];
        while let Some((index, elm)) = self.pick(loc, last_elem){
           self.eaten.push((loc.clone(),index)); 
           component_chain.push(elm);
        }
        component_chain
    }

    /// pick which element that can chain to the last element specified
    /// reverse the each element if necessary
    /// return the match element and its relative position from the component elements
    fn pick(&self, loc: &Loc, last_elm: &Element)-> Option<(usize, Element)>{
        match self.get(loc){
            Some(elements) => {
                for i in 0..elements.len(){
                    if !self.eaten(loc, i){
                        match last_elm.try_chain(&elements[i]){
                            Some(chained) => {
                                return Some((i, chained));
                            }
                            None => continue
                        };
                    };
                }
                None
            },
            None => None
        }
    }



    /// check the specific element at this location has been picked or not
    fn eaten(&self, loc: &Loc, index: usize) -> bool {
        self.eaten.iter()
            .find(|&&(ref l, i)| l == loc &&  i == index)
            .map_or(false, |_| true)
    }

    fn half_eaten(&self, loc: &Loc) -> bool {
        self.eaten.iter()
            .find(|&&(ref l, i)| l == loc )
            .map_or(false, |_| true)
    }


    fn get(&self, loc: &Loc) -> Option<&Vec<Element>> {
        let found = self.elements
            .iter()
            .find(|x| {
                let &(ref l, _) = *x;
                loc == l
            });
        match found {
            Some(&(_, ref elm)) => Some(elm),    
            None => None,
        }
    }


    // TODO: order the elements in such a way that
    // the start -> end -> start chains nicely
    pub fn optimize(elements: &Vec<(Loc, Vec<Element>)>, settings: &Settings) -> Vec<Element> {
        let mut optimizer = Optimizer::new(elements.clone());
        let mut phase1:Vec<(&Loc,Vec<Element>)> = vec![];
        for &(ref loc, ref elem) in elements {
            let traced = optimizer.trace(loc, elem);
            phase1.push((loc, traced));
        }
        let mut optimized:Vec<Element> = vec![];
        for (loc,ph_elements) in phase1{
            for i in 0..ph_elements.len(){
                if !optimizer.eaten(loc,i){
                    optimized.push(ph_elements[0].clone());
                }else{
                    println!("skipping {:?}", loc);
                }
            }
        }
        optimized
    }
    // take all paths and non-arrow line in 1 path
    // the text in separated
    fn merge_paths(&self, elements: &Vec<Element>) -> Vec<Element> {
        let mut merged = vec![];
        let mut solid_paths = vec![];
        let mut dashed_paths = vec![];
        let mut arrows = vec![];
        let mut text = vec![];
        for elm in elements {
            match *elm {
                Element::Line(_, _, ref stroke, ref feature) => {
                    match *feature {
                        Feature::Arrow => {
                            arrows.push(elm.clone());
                        },
                        Feature::Circle =>{
                            arrows.push(elm.clone());
                        },
                        Feature::Nothing => {
                            match *stroke {
                                Stroke::Solid => {
                                    solid_paths.push(elm.clone());
                                }
                                Stroke::Dashed => {
                                    dashed_paths.push(elm.clone());
                                }
                            }
                        }
                    }
                }
                Element::Arc(_, _, _, _) => solid_paths.push(elm.clone()),
                Element::Text(_, _) => text.push(elm.clone()),
                Element::Path(_, _, _, _) => {
                    merged.push(elm.clone());
                }
            }
        }
        merged.push(unify(&solid_paths, Stroke::Solid));
        merged.push(unify(&dashed_paths, Stroke::Dashed));
        merged.extend(arrows);
        merged.extend(text);
        merged
    }
}

fn unify(elements: &Vec<Element>, stroke: Stroke) -> Element {
    let mut paths = String::new();
    let mut last_loc = None;
    let mut start = None;
    for elm in elements {
        match *elm {
            Element::Line(ref s, ref e, _, _) => {
                if start.is_none() {
                    start = Some(s.clone());
                }
                let match_last_loc = match last_loc {
                    Some(last_loc) => *s == last_loc,
                    None => false,
                };
                if match_last_loc {
                    paths.push_str(&format!(" L {} {}", e.x, e.y));
                } else {
                    paths.push_str(&format!(" M {} {} L {} {}", s.x, s.y, e.x, e.y));
                }
                last_loc = Some(e.clone());
            }
            Element::Arc(ref s, ref e, r, sw) => {
                if start.is_none() {
                    start = Some(s.clone());
                }
                let match_last_loc = match last_loc {
                    Some(last_loc) => *s == last_loc,
                    None => false,
                };
                let sweep = if sw { 1 } else { 0 };
                if match_last_loc {
                    paths.push_str(&format!(" A {} {} 0 0 {} {} {}", r, r, sweep, e.x, e.y));
                } else {
                    paths.push_str(&format!(" M {} {} A {} {} 0 0 {} {} {}",
                                            s.x,
                                            s.y,
                                            r,
                                            r,
                                            sweep,
                                            e.x,
                                            e.y));
                }
                last_loc = Some(e.clone());
            }
            _ => panic!("only lines are arc can be unified"),
        }
    }
    let el_start = match start {
        Some(start) => start.clone(),
        None => Point::new(0.0, 0.0),
    };
    let el_end = match last_loc {
        Some(last_loc) => last_loc.clone(),
        None => Point::new(0.0, 0.0),
    };
    Element::Path(el_start, el_end, paths, stroke)
}
