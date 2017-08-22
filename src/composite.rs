use expr;
use types;
use types::{SortKind};
use embed::Embed;
use std::cmp::{Ordering,min,max};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::marker::PhantomData;
use std::rc::Rc;
use std::cell;
use std::cell::RefCell;

pub trait Composite : Sized + 'static {
    
    fn num_elem(&self) -> usize;
    fn elem_sort<Em : Embed>(&self,usize,&mut Em)
                             -> Result<Em::Sort,Em::Error>;

    fn combine(&self,&Self) -> Option<Self>;

    fn combine_elem<FComb,FL,FR,Acc>(&self,&Self,&FComb,&FL,&FR,Acc,&mut usize,&mut usize,&mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc;

    fn invariant<Em : Embed,F>(&self,&mut Em,&F,&mut usize,&mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {
        Ok(())
    }
}

pub struct CompExpr<C : Composite> {
    pub expr: Box<expr::Expr<types::Sort,u64,CompExpr<C>,()>>,
    phantom: PhantomData<C>
}

pub struct Singleton(types::Sort);

impl Composite for Singleton {
    fn num_elem(&self) -> usize { 1 }
    fn elem_sort<Em : Embed>(&self,_:usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        self.0.embed(em)
    }
    fn combine(&self,oth: &Self) -> Option<Self> {
        if self.0==oth.0 {
            None
        } else {
            Some(Singleton(self.0.clone()))
        }
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,_: &Self,f: &FComb,_: &FL,_: &FR,acc: Acc,offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {
        let nacc = f(acc,*offl,*offr,*offn);
        *offl+=1;
        *offr+=1;
        *offn+=1;
        nacc
    }
}

pub struct SingletonBool {}

pub static BOOL_SINGLETON : SingletonBool = SingletonBool {};

impl Composite for SingletonBool {
    fn num_elem(&self) -> usize { 1 }
    fn elem_sort<Em : Embed>(&self,_:usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        em.tp_bool()
    }
    fn combine(&self,_: &Self) -> Option<Self> {
        Some(SingletonBool {})
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,_: &Self,f: &FComb,_: &FL,_: &FR,acc: Acc,offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {
        let nacc = f(acc,*offl,*offr,*offn);
        *offl+=1;
        *offr+=1;
        *offn+=1;
        nacc
    }
}

impl<T : Composite + Clone> Composite for Vec<T> {
    fn num_elem(&self) -> usize {
        let mut acc = 0;
        for el in self.iter() {
            acc+=el.num_elem()
        }
        acc
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        let mut acc = 0;
        for el in self.iter() {
            let num = el.num_elem();
            if acc+num > n {
                return el.elem_sort(n-acc,em)
            }
            acc+=num;
        }
        panic!("Invalid index {}",n)
    }
    fn combine(&self,oth: &Vec<T>) -> Option<Vec<T>> {
        let ssize = self.len();
        let osize = oth.len();
        let min_size = min(ssize,osize);
        let max_size = max(ssize,osize);

        let mut res = Vec::with_capacity(max_size);
        
        for i in 0..min_size {
            match self[i].combine(&oth[i]) {
                None => return None,
                Some(nel) => { res.push(nel) }
            }
        }

        if ssize > osize {
            for i in osize..ssize {
                res.push(self[i].clone())
            }
        } else if osize > ssize {
            for i in ssize..osize {
                res.push(oth[i].clone())
            }
        }
        Some(res)
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,
                                     oth: &Vec<T>,
                                     comb: &FComb,
                                     onlyl: &FL,
                                     onlyr: &FR,
                                     acc: Acc,
                                     offl: &mut usize,
                                     offr: &mut usize,
                                     offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {

        let ssize = self.len();
        let osize = oth.len();
        let min_size = min(ssize,osize);
        let mut cacc = acc;
        
        for i in 0..min_size {
            cacc = self[i].combine_elem(&oth[i],comb,onlyl,onlyr,cacc,offl,offr,offn)
        }

        if ssize > osize {
            for i in osize..ssize {
                for _ in 0..self[i].num_elem() {
                    cacc = onlyl(cacc,*offl,*offn);
                    *offl += 1;
                    *offn += 1;
                }
            }
        } else if osize > ssize {
            for i in ssize..osize {
                for _ in 0..oth[i].num_elem() {
                    cacc = onlyl(cacc,*offl,*offn);
                    *offl += 1;
                    *offn += 1;
                }
            }
        }
        cacc
    }
    fn invariant<Em : Embed,F>(&self,em: &mut Em,f: &F,off: &mut usize,res: &mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {

        for el in self.iter() {
            el.invariant(em,f,off,res)?;
        }
        Ok(())
    }
}

pub struct Choice<T>(Vec<T>);

impl<T : Composite + Ord + Clone> Composite for Choice<T> {
    fn num_elem(&self) -> usize {
        let mut acc = 0;
        for el in self.0.iter() {
            acc+=el.num_elem()+1
        }
        acc
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        let mut acc = 0;
        for el in self.0.iter() {
            if n==acc {
                return em.embed_sort(SortKind::Bool)
            }
            acc+=1;
            let num = el.num_elem();
            if acc+num > n {
                return el.elem_sort(n-acc,em)
            }
            acc+=num;
        }
        panic!("Invalid index {}",n)
    }
    fn combine(&self,oth: &Choice<T>) -> Option<Choice<T>> {
        let mut offs = 0;
        let mut offo = 0;
        let mut res = Vec::new();
        loop {
            if offs >= self.0.len() {
                for i in offo..oth.0.len() {
                    res.push(oth.0[i].clone())
                }
                break
            }
            if offo >= oth.0.len() {
                for i in offs..self.0.len() {
                    res.push(self.0[i].clone())
                }
                break
            }
            match self.0[offs].cmp(&oth.0[offo]) {
                Ordering::Equal => match self.0[offs].combine(&oth.0[offo]) {
                    None => return None,
                    Some(nel) => {
                        offs+=1;
                        offo+=1;
                        res.push(nel);
                    }
                },
                Ordering::Less => {
                    offs+=1;
                    res.push(self.0[offs].clone());
                },
                Ordering::Greater => {
                    offo+=1;
                    res.push(oth.0[offo].clone());
                }
            }
        }
        Some(Choice(res))
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,
                                     oth: &Choice<T>,
                                     comb: &FComb,
                                     onlyl: &FL,
                                     onlyr: &FR,
                                     acc: Acc,
                                     offl: &mut usize,
                                     offr: &mut usize,
                                     offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {

        let mut offs = 0;
        let mut offo = 0;
        let mut cacc = acc;

        loop {
            if offs >= self.0.len() {
                for i in offo..oth.0.len() {
                    for _ in 0..oth.0[i].num_elem()+1 {
                        cacc = onlyr(cacc,*offr,*offn);
                        *offr+=1;
                        *offn+=1;
                    }
                }
                break;
            }
            if offo >= oth.0.len() {
                for i in offs..self.0.len() {
                    for _ in 0..self.0[i].num_elem()+1 {
                        cacc = onlyl(cacc,*offl,*offn);
                        *offl+=1;
                        *offn+=1;
                    }
                }
                break;
            }
            let ref l = self.0[offs];
            let ref r = oth.0[offo];
            match l.cmp(&r) {
                Ordering::Equal => {
                    cacc = comb(cacc,*offl,*offr,*offn);
                    *offl+=1;
                    *offr+=1;
                    *offn+=1;
                    cacc = l.combine_elem(&r,comb,onlyl,onlyr,cacc,offl,offr,offn);
                    offs+=1;
                    offo+=1;
                },
                Ordering::Less => {
                    for _ in 0..l.num_elem()+1 {
                        cacc = onlyl(cacc,*offl,*offn);
                        *offl+=1;
                        *offn+=1;
                    }
                    offs+=1;
                },
                Ordering::Greater => {
                    for _ in 0..r.num_elem()+1 {
                        cacc = onlyr(cacc,*offr,*offn);
                        *offr+=1;
                        *offn+=1;
                    }
                    offo+=1;
                }
            }
        }
        cacc
    }
    fn invariant<Em : Embed,F>(&self,em: &mut Em,f: &F,off: &mut usize,res: &mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {

        let mut selectors = Vec::with_capacity(self.0.len());

        for el in self.0.iter() {
            let sel = f(*off,em)?;
            
            *off+=1;

            let last_pos = res.len();
            el.invariant(em,f,off,res)?;
            for i in last_pos..res.len() {
                let new = em.embed(expr::Expr::App(expr::Function::Implies(2),vec![sel.clone(),res[i].clone()]))?;
                res[i] = new;
            }
            
            selectors.push(sel);
        }

        let inv1 = em.embed(expr::Expr::App(expr::Function::AtMost(1,selectors.len()),selectors.clone()))?;
        res.push(inv1);
        let inv2 = em.embed(expr::Expr::App(expr::Function::AtLeast(1,selectors.len()),selectors))?;
        res.push(inv2);
        Ok(())
    }

}

impl<K : Ord + Clone + 'static,T : Composite + Clone> Composite for BTreeMap<K,T> {
    fn num_elem(&self) -> usize {
        let mut acc = 0;
        for v in self.values() {
            acc+=v.num_elem();
        }
        acc
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        let mut acc = 0;
        for v in self.values() {
            let sz = v.num_elem();
            if acc+sz > n {
                return v.elem_sort(n-acc,em)
            }
            acc+=sz;
        }
        panic!("Invalid index: {}",n)
    }
    fn combine(&self,oth: &BTreeMap<K,T>) -> Option<BTreeMap<K,T>> {
        let mut res = (*self).clone();
        for (k,v) in oth.iter() {
            match res.entry(k.clone()) {
                Entry::Occupied(mut e) => match e.get().combine(v) {
                    None => return None,
                    Some(nv) => { e.insert(nv); }
                },
                Entry::Vacant(e) => { e.insert(v.clone()) ; }
            }
        }
        Some(res)
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,oth: &BTreeMap<K,T>,
                                     comb: &FComb,onlyl: &FL,onlyr: &FR,
                                     acc: Acc,
                                     offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {

        let mut cacc = acc;
        let mut iter_l = self.iter();
        let mut iter_r = oth.iter();
        let mut cur_l = None;
        let mut cur_r : Option<(&K,&T)> = None;

        loop {
            let (key_l,el_l) = match cur_l {
                None => match iter_l.next() {
                    None => {
                        match cur_r {
                            None => {},
                            Some((_,el)) => for _ in 0..el.num_elem() {
                                cacc = onlyr(cacc,*offr,*offn);
                                *offr+=1;
                                *offn+=1;
                            }
                        }
                        for (_,el) in iter_r {
                            for _ in 0..el.num_elem() {
                                cacc = onlyr(cacc,*offr,*offn);
                                *offr+=1;
                                *offn+=1;
                            }
                        }
                        return cacc
                    },
                    Some(el) => el
                },
                Some(el) => el
            };
            let (key_r,el_r) = match cur_r {
                None => match iter_r.next() {
                    None => {
                        for _ in 0..el_l.num_elem() {
                            cacc = onlyl(cacc,*offl,*offn);
                            *offl+=1;
                            *offn+=1;
                        }
                        for (_,el) in iter_l {
                            for _ in 0..el.num_elem() {
                                cacc = onlyl(cacc,*offl,*offn);
                                *offl+=1;
                                *offn+=1;
                            }
                        }
                        return cacc
                    },
                    Some(el) => el
                },
                Some(el) => el
            };
            match key_l.cmp(key_r) {
                Ordering::Equal => {
                    cacc = el_l.combine_elem(el_r,comb,onlyl,onlyr,cacc,offl,offr,offn);
                    cur_l = None;
                    cur_r = None;
                },
                Ordering::Less => {
                    for _ in 0..el_l.num_elem() {
                        cacc = onlyl(cacc,*offl,*offn);
                        *offl+=1;
                        *offn+=1;
                    }
                    cur_l = None;
                    cur_r = Some((key_r,el_r));
                },
                Ordering::Greater => {
                    for _ in 0..el_r.num_elem() {
                        cacc = onlyr(cacc,*offr,*offn);
                        *offr+=1;
                        *offn+=1;
                    }
                    cur_l = Some((key_l,el_r));
                    cur_r = None;
                }
            }
        }
    }
    fn invariant<Em : Embed,F>(&self,em: &mut Em,f: &F,off: &mut usize,res: &mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {

        for el in self.values() {
            el.invariant(em,f,off,res)?;
        }
        Ok(())
    }
}

impl<T : Composite + Clone> Composite for Option<T> {
    fn num_elem(&self) -> usize {
        match *self {
            None => 0,
            Some(ref c) => c.num_elem()
        }
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        match *self {
            None => panic!("Invalid index: {}",n),
            Some(ref c) => c.elem_sort(n,em)
        }
    }
    fn combine(&self,oth: &Option<T>) -> Option<Option<T>> {
        match *self {
            None => Some((*oth).clone()),
            Some(ref c1) => match *oth {
                None => Some(Some(c1.clone())),
                Some(ref c2) => match c1.combine(&c2) {
                    None => None,
                    Some(r) => Some(Some(r))
                }
            }
        }
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,oth: &Option<T>,
                                     comb: &FComb,onlyl: &FL,onlyr: &FR,
                                     acc: Acc,offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {

        match *self {
            None => match *oth {
                None => acc,
                Some(ref el) => {
                    let mut cacc = acc;
                    for _ in 0..el.num_elem() {
                        cacc = onlyr(cacc,*offr,*offn);
                        *offr+=1;
                        *offl+=1;
                    }
                    cacc
                }
            },
            Some(ref el1) => match *oth {
                None => {
                    let mut cacc = acc;
                    for _ in 0..el1.num_elem() {
                        cacc = onlyl(cacc,*offl,*offn);
                        *offl+=1;
                        *offr+=1;
                    }
                    cacc
                },
                Some(ref el2) => el1.combine_elem(el2,comb,onlyl,onlyr,acc,offl,offr,offn)
            }
        }
    }
    fn invariant<Em : Embed,F>(&self,em: &mut Em,f: &F,off: &mut usize,res: &mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {
        match *self {
            None => Ok(()),
            Some(ref el) => el.invariant(em,f,off,res)
        }
    }
}

pub struct Array<Idx : Composite,T : Composite> {
    index: Idx,
    element: T
}

impl<Idx : Composite + Eq + Clone,T : Composite> Composite for Array<Idx,T> {
    fn num_elem(&self) -> usize {
        self.element.num_elem()
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        let srt = self.element.elem_sort(n,em)?;
        let idx_sz = self.index.num_elem();
        let mut idx_arr = Vec::with_capacity(idx_sz as usize);
        for i in 0..idx_sz {
            idx_arr.push(self.index.elem_sort(i,em)?);
        }
        em.embed_sort(SortKind::Array(idx_arr,srt))
    }
    fn combine(&self,oth: &Array<Idx,T>) -> Option<Array<Idx,T>> {
        if self.index!=oth.index {
            return None
        }
        match self.element.combine(&oth.element) {
            None => None,
            Some(nel) => Some(Array { index: self.index.clone(),
                                      element: nel })
        }
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,oth: &Array<Idx,T>,
                                     comb: &FComb,onlyl: &FL,onlyr: &FR,
                                     acc: Acc,offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {
        self.element.combine_elem(&oth.element,comb,onlyl,onlyr,
                                  acc,offl,offr,offn)
    }
    // FIXME: Forall invariants
}

impl Composite for () {
    fn num_elem(&self) -> usize { 0 }
    fn elem_sort<Em : Embed>(&self,n: usize,_: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        panic!("Invalid index: {}",n)
    }
    fn combine(&self,_:&()) -> Option<()> {
        Some(())
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,_:&(),_:&FComb,_:&FL,_:&FR,
                                     acc:Acc,_:&mut usize,_:&mut usize,_:&mut usize)
                                     -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {
        acc
    }
        
}

impl<A : Composite,B : Composite> Composite for (A,B) {
    fn num_elem(&self) -> usize {
        self.0.num_elem() + self.1.num_elem()
    }
    fn elem_sort<Em : Embed>(&self,n: usize,em: &mut Em)
                             -> Result<Em::Sort,Em::Error> {
        let sz0 = self.0.num_elem();
        if n>=sz0 {
            self.1.elem_sort(n-sz0,em)
        } else {
            self.0.elem_sort(n,em)
        }
    }
    fn combine(&self,oth: &(A,B)) -> Option<(A,B)> {
        match self.0.combine(&oth.0) {
            None => None,
            Some(n0) => match self.1.combine(&oth.1) {
                None => None,
                Some(n1) => Some((n0,n1))
            }
        }
    }
    fn combine_elem<FComb,FL,FR,Acc>(&self,oth: &(A,B),
                                     comb: &FComb,onlyl: &FL,onlyr: &FR,
                                     acc: Acc,offl: &mut usize,offr: &mut usize,offn: &mut usize) -> Acc
        where FComb : Fn(Acc,usize,usize,usize) -> Acc, FL : Fn(Acc,usize,usize) -> Acc, FR : Fn(Acc,usize,usize) -> Acc {
        let acc1 = self.0.combine_elem(&oth.0,comb,onlyl,onlyr,
                                       acc,offl,offr,offn);
        self.1.combine_elem(&oth.1,comb,onlyl,onlyr,
                            acc1,offl,offr,offn)
    }
    fn invariant<Em : Embed,F>(&self,em: &mut Em,f: &F,off: &mut usize,res: &mut Vec<Em::Expr>)
                               -> Result<(),Em::Error>
        where F : Fn(usize,&mut Em) -> Result<Em::Expr,Em::Error> {

        self.0.invariant(em,f,off,res)?;
        self.1.invariant(em,f,off,res)
    }

}

pub trait GetElem<Em : Embed> {
    fn get_elem(&self,usize,&mut Em) -> Result<Em::Expr,Em::Error>;
}

pub struct OffsetGetter<Em : Embed> {
    offset: usize,
    getter: Box<GetElem<Em>>,
}

impl<Em : Embed> GetElem<Em> for OffsetGetter<Em> {
    fn get_elem(&self,n: usize,em: &mut Em)
                -> Result<Em::Expr,Em::Error> {
        self.getter.get_elem(n+self.offset,em)
    }
}

pub enum OptRef<'a,T : 'a> {
    Ref(&'a T),
    Owned(T)
}

impl<'a,T : 'a + Clone> OptRef<'a,T> {
    pub fn as_ref(&'a self) -> &'a T {
        match *self {
            OptRef::Ref(r) => r,
            OptRef::Owned(ref x) => x
        }
    }
    pub fn as_obj(self) -> T {
        match self {
            OptRef::Ref(x) => (*x).clone(),
            OptRef::Owned(x) => x
        }
    }
}

pub enum Transformation<Em : Embed> {
    Id(usize),
    View(usize,usize,Rc<Transformation<Em>>), // View with an offset and size
    Concat(usize,Vec<Rc<Transformation<Em>>>), // Record size to prevent recursion
    Constant(Vec<Em::Expr>),
    Map(usize, // Resulting size
        Box<Fn(&[Em::Expr],&mut Em) -> Vec<Em::Expr>>, // mapping function
        Rc<Transformation<Em>>, // transformation
        RefCell<Option<Vec<Em::Expr>>> // cache
    ),
    Write(usize, // Resulting size
          usize, // Write offset
          usize, // Previous size
          Rc<Transformation<Em>>, // Write source
          Rc<Transformation<Em>> // Write target
    ),
    MapByElem(Box<for <'a,'b> Fn(&'a [Em::Expr],usize,Em::Expr,&'b mut Em) -> Result<Em::Expr,Em::Error>>,
              Rc<Transformation<Em>>)
}

enum BorrowedSlice<'a,T : 'a> {
    BorrowedSlice(&'a [T]),
    CachedSlice(cell::Ref<'a,Vec<T>>,usize,usize),
    OwnedSlice(Vec<T>)
}

impl<'a,T : 'a> BorrowedSlice<'a,T> {
    fn get(&'a self) -> &'a [T] {
        match *self {
            BorrowedSlice::BorrowedSlice(sl) => sl,
            BorrowedSlice::CachedSlice(ref sl,start,end) => &(*sl)[start..end],
            BorrowedSlice::OwnedSlice(ref sl) => &sl[..]
        }
    }
}

impl<Em : Embed> Transformation<Em> {
    pub fn view(off: usize,len: usize,t: Rc<Transformation<Em>>) -> Rc<Transformation<Em>> {
        if len==0 {
            Rc::new(Transformation::Id(0))
        } else if off==0 && t.size()==len {
            t
        } else {
            Rc::new(Transformation::View(off,len,t))
        }
    }
    pub fn concat(trs: &[Rc<Transformation<Em>>]) -> Rc<Transformation<Em>> {
        let mut only_one = None;
        let mut none = true;
        let mut req_alloc = 0;
        let mut sz = 0;
        for tr in trs.iter() {
            if tr.size()==0 {
                continue
            }
            match **tr {
                Transformation::Concat(nsz,ref ntrs) => {
                    sz+=nsz;
                    req_alloc+=ntrs.len();
                },
                _ => {
                    sz+=tr.size();
                    req_alloc+=1;
                }
            }
            only_one = if none { Some(tr) } else { None };
            none = false;
        }
        if none {
            return Rc::new(Transformation::Id(0));
        }
        if let Some(only) = only_one {
            return only.clone()
        }
        let mut rvec = Vec::with_capacity(req_alloc);
        for tr in trs.iter() {
            if tr.size()==0 {
                continue
            }
            match **tr {
                Transformation::Concat(_,ref ntrs) => {
                    rvec.extend_from_slice(&ntrs[..]);
                },
                _ => {
                    rvec.push(tr.clone());
                }
            }
        }
        Rc::new(Transformation::Concat(sz,rvec))
    }
    pub fn size(&self) -> usize {
        match *self {
            Transformation::Id(sz) => sz,
            Transformation::View(_,nsz,_) => nsz,
            Transformation::Concat(sz,_) => sz,
            Transformation::Constant(ref vec) => vec.len(),
            Transformation::Map(sz,_,_,_) => sz,
            Transformation::Write(sz,_,_,_,_) => sz,
            Transformation::MapByElem(_,ref tr) => tr.size()
        }
    }
    pub fn clear_cache(&self) -> () {
        match *self {
            Transformation::Id(_) => (),
            Transformation::View(_,_,ref tr) => tr.clear_cache(),
            Transformation::Concat(_,ref vec) => for el in vec.iter() {
                el.clear_cache()
            },
            Transformation::Constant(_) => (),
            Transformation::Map(_,_,ref tr,ref cache) => {
                tr.clear_cache();
                *cache.borrow_mut() = None;
            },
            Transformation::Write(_,_,_,ref obj,ref trg) => {
                obj.clear_cache();
                trg.clear_cache();
            },
            Transformation::MapByElem(_,ref tr) => tr.clear_cache()
        }
    }
    fn as_slice<'a>(&'a self,arr: &'a [Em::Expr],off: usize,len: usize)
                    -> Option<BorrowedSlice<'a,Em::Expr>> {
        match *self {
            Transformation::Id(_) => Some(BorrowedSlice::BorrowedSlice(&arr[off..off+len])),
            Transformation::View(noff,_,ref tr) => tr.as_slice(arr,off+noff,len),
            Transformation::Concat(_,ref vec) => {
                let mut acc = 0;
                for el in vec.iter() {
                    let sz = el.size();
                    if off < acc+sz {
                        if sz<=len {
                            return el.as_slice(arr,off-acc,len)
                        } else {
                            return None
                        }
                    }
                    acc+=sz;
                }
                panic!("Invalid index: {}",off)
            },
            Transformation::Constant(ref vec) => Some(BorrowedSlice::BorrowedSlice(&vec[off..off+len])),
            Transformation::Map(_,_,_,ref cache) => {
                let cache_ref : cell::Ref<Option<Vec<Em::Expr>>> = cache.borrow();
                match *cache_ref {
                    None => None,
                    Some(_) => {
                        let vec_ref : cell::Ref<Vec<Em::Expr>> = cell::Ref::map(cache_ref,|x| match x {
                            &Some(ref x) => x,
                            &None => unreachable!()
                        });
                        Some(BorrowedSlice::CachedSlice(vec_ref,off,off+len))
                    }
                }
            },
            Transformation::Write(_,wr_off,repl_sz,ref obj,ref trg) => if off+len <= wr_off {
                trg.as_slice(arr,off,len)
            } else if off >= wr_off && off+len <= wr_off+obj.size() {
                obj.as_slice(arr,off-wr_off,len)
            } else if off >= wr_off+obj.size() {
                trg.as_slice(arr,off-obj.size()+repl_sz,len)
            } else {
                None
            },
            _ => None
        }
    }
    fn to_slice<'a>(&'a self,arr: &'a [Em::Expr],off: usize,len: usize,em: &mut Em)
                    -> Result<BorrowedSlice<'a,Em::Expr>,Em::Error> {
        match self.as_slice(arr,off,len) {
            Some(res) => Ok(res),
            None => {
                let mut rvec = Vec::with_capacity(len);
                for i in 0..len {
                    rvec.push(self.get(arr,off+i,em)?);
                }
                Ok(BorrowedSlice::OwnedSlice(rvec))
            }
        }
    }
    pub fn get(&self,arr: &[Em::Expr],idx: usize,em: &mut Em) -> Result<Em::Expr,Em::Error> {
        match *self {
            Transformation::Id(_) => Ok(arr[idx].clone()),
            Transformation::View(off,_,ref tr)
                => tr.get(arr,off+idx,em),
            Transformation::Concat(_,ref vec) => {
                let mut acc = 0;
                for el in vec.iter() {
                    let sz = el.size();
                    if idx < acc+sz {
                        return el.get(arr,idx-acc,em)
                    }
                    acc+=sz;
                }
                panic!("Invalid index: {}",idx)
            },
            Transformation::Constant(ref vec) => Ok(vec[idx].clone()),
            Transformation::Map(_,ref f,ref tr,ref cache) => {
                let mut cache_ref : cell::RefMut<Option<Vec<Em::Expr>>> = (*cache).borrow_mut();
                match *cache_ref {
                    Some(ref rcache) => return Ok(rcache[idx].clone()),
                    None => {}
                }
                let sl = tr.to_slice(arr,0,arr.len(),em)?;
                let narr = f(sl.get(),em);
                let res = narr[idx].clone();
                *cache_ref = Some(narr);
                return Ok(res)
            },
            Transformation::Write(_,wr_off,repl_sz,ref obj,ref trg) => if idx < wr_off {
                trg.get(arr,idx,em)
            } else if idx >= wr_off && idx < wr_off+obj.size() {
                obj.get(arr,idx-wr_off,em)
            } else {
                trg.get(arr,idx-obj.size()+repl_sz,em)
            },
            Transformation::MapByElem(ref f,ref tr)
                => f(arr,idx,tr.get(arr,idx,em)?,em)
        }
    }
}

pub trait Transition<Src : Composite,Trg : Composite> {

    fn apply<'a,Em : 'static + Embed>
        (&self,OptRef<'a,Src>,Rc<Transformation<Em>>,&mut Em)
         -> Result<(OptRef<'a,Trg>,
                    Rc<Transformation<Em>>),Em::Error>;
}

pub struct Seq<A : Composite,B : Composite,C : Composite,
               T1 : Transition<A,B>,
               T2 : Transition<B,C>> {
    t1: T1,
    t2: T2,
    phantom: PhantomData<(A,B,C)>,
}

impl<A : Composite, B : Composite, C : Composite,
     T1 : Transition<A,B>,
     T2 : Transition<B,C>
     > Transition<A,C> for Seq<A,B,C,T1,T2> {

    fn apply<'a,Em : 'static + Embed>(&self,a: OptRef<'a,A>,
                                      get_a: Rc<Transformation<Em>>,em: &mut Em)
                                      -> Result<(OptRef<'a,C>,
                                                 Rc<Transformation<Em>>),Em::Error> {
        let (b,get_b) = self.t1.apply(a,get_a,em)?;
        self.t2.apply(b,get_b,em)
    }
}

pub struct GetVecElem {
    which: usize
}

impl<T : Composite + Clone> Transition<Vec<T>,T> for GetVecElem {

    fn apply<'a,Em : 'static + Embed>(&self,vec: OptRef<'a,Vec<T>>,
                                      inp: Rc<Transformation<Em>>,_: &mut Em)
                                      -> Result<(OptRef<'a,T>,Rc<Transformation<Em>>),
                                                Em::Error> {
        match vec {
            OptRef::Ref(rvec) => {
                let mut off = 0;
                for el in rvec.iter().take(self.which) {
                    off+=el.num_elem();
                }
                let len = rvec[self.which].num_elem();
                Ok((OptRef::Ref(&rvec[self.which]),
                    Transformation::view(off,len as usize,inp)))
            },
            OptRef::Owned(mut rvec) => {
                let mut off = 0;
                for el in rvec.iter().take(self.which) {
                    off+=el.num_elem();
                }
                let len = rvec[self.which].num_elem();
                Ok((OptRef::Owned(rvec.remove(self.which)),
                    Transformation::view(off as usize,len as usize,inp)))
            }
        }
    }
}

pub struct SetVecElem {
    which: usize
}

impl<T : Composite + Clone> Transition<(Vec<T>,T),Vec<T>> for SetVecElem {

    fn apply<'a,Em : 'static + Embed>(&self,args: OptRef<'a,(Vec<T>,T)>,
                                      inp: Rc<Transformation<Em>>,_:&mut Em)
                                      -> Result<(OptRef<'a,Vec<T>>,Rc<Transformation<Em>>),Em::Error> {
        match args {
            OptRef::Ref(&(ref vec,ref el)) => {
                let vlen = vec.num_elem();
                let mut off_store = 0;
                for el in vec.iter().take(self.which) {
                    off_store+=el.num_elem();
                }
                let old = vec[self.which].num_elem();
                let new = el.num_elem();
                let mut rvec = vec.clone();
                rvec[self.which] = el.clone();
                Ok((OptRef::Owned(rvec),
                    Transformation::concat(&[Transformation::view(0,off_store,inp.clone()),
                                             Transformation::view(vlen,new,inp.clone()),
                                             Transformation::view(off_store+old,
                                                                  vlen-off_store-old,inp)])))
            },
            OptRef::Owned((mut vec,el)) => {
                let vlen = vec.num_elem();
                let mut off_store = 0;
                for el in vec.iter().take(self.which) {
                    off_store+=el.num_elem();
                }
                let old = vec[self.which].num_elem();
                let new = el.num_elem();
                vec[self.which] = el;
                Ok((OptRef::Owned(vec),
                    Transformation::concat(&[Transformation::view(0,off_store,inp.clone()),
                                             Transformation::view(vlen,new,inp.clone()),
                                             Transformation::view(off_store+old,
                                                                  vlen-off_store-old,inp)])))
            }
        }
    }
}

pub struct GetArrayElem {}

impl<Idx : Composite + Eq + Clone, T : Composite
     > Transition<(Array<Idx,T>,Idx),T> for GetArrayElem {
    fn apply<'a,Em : 'static + Embed>(&self,args: OptRef<'a,(Array<Idx,T>,Idx)>,
                                      inp: Rc<Transformation<Em>>,_: &mut Em)
                                      -> Result<(OptRef<'a,T>,Rc<Transformation<Em>>),Em::Error> {
        match args {
            OptRef::Owned((arr,idx)) => {
                assert!(arr.index==idx);
                let off_idx = arr.element.num_elem();
                let num_idx = idx.num_elem();
                let arr_elems = Transformation::view(0,off_idx,inp.clone());
                let fun = move |carr: &[Em::Expr],_: usize,e: Em::Expr,em: &mut Em| -> Result<Em::Expr,Em::Error> {
                    let mut rvec = Vec::with_capacity(num_idx);
                    for i in 0..num_idx {
                        let idx_el = inp.get(carr,off_idx+i,em)?;
                        rvec.push(idx_el);
                    }
                    em.select(e,rvec)
                };
                let mp = Rc::new(Transformation::MapByElem(Box::new(fun),arr_elems));
                Ok((OptRef::Owned(arr.element),mp))
            },
            OptRef::Ref(&(ref arr,ref idx)) => {
                assert!(arr.index==*idx);
                let off_idx = arr.element.num_elem();
                let num_idx = idx.num_elem();
                let arr_elems = Transformation::view(0,off_idx,inp.clone());
                let fun = move |carr: &[Em::Expr],_: usize,e: Em::Expr,em: &mut Em| -> Result<Em::Expr,Em::Error> {
                    let mut rvec = Vec::with_capacity(num_idx);
                    for i in 0..num_idx {
                        let idx_el = inp.get(carr,off_idx+i,em)?;
                        rvec.push(idx_el);
                    }
                    em.select(e,rvec)
                };
                let mp = Rc::new(Transformation::MapByElem(Box::new(fun),arr_elems));
                Ok((OptRef::Ref(&arr.element),mp))
            }
        }
        
    }
        
}
