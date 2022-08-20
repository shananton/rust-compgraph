use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::internals::InvalidateCacheMut;

define_nodes! {
    add(a, b) { a + b }
    mul(a, b) { a * b }
    sin(x) { x.sin() }
    pow_f32(x, e) { x.powf(e) }
    pub add3(a, b, c) { a + b + c } // test parsing of `pub` in macro
}

fn round(x: f32, precision: u32) -> f32 {
    let m = 10i32.pow(precision) as f32;
    (x * m).round() / m
}

#[test]
fn constant() {
    let graph = add(
        3.0,
        mul(2.0, 7.0)
    );
    assert_eq!(graph.compute(), 17.0);
}

#[test]
fn example_from_pdf() {
    let x1 = create_input();
    let x2 = create_input();
    let x3 = create_input();

    let graph = add(
        x1.clone(),
        mul(
            x2.clone(),
            sin(
                add(
                    x2.clone(),
                    pow_f32(x3.clone(), 3f32)
                )
            )
        )
    );
    x1.set(1f32);
    x2.set(2f32);
    x3.set(3f32);

    let mut result = graph.compute();
    result = round(result, 5);
    assert_eq!(round(result, 5), -0.32727);

    x1.set(2f32);
    x2.set(3f32);
    x3.set(4f32);
    result = graph.compute();
    result = round(result, 5);
    assert_eq!(round(result, 5), -0.56656);
}

#[test]
fn cache_invalidation() {
    let x1 = create_input();
    let x2 = create_input();
    let x3 = create_input();
    x1.set(0.0);
    x2.set(0.0);
    x3.set(0.0);
    let y1 = sin(x1.clone());
    let y2 = mul(y1.clone(), x2.clone());
    let y3 = add3(y1.clone(), y2.clone(), x3.clone());

    struct InvalidateMockSubscriber {
        invalidate_count: u32
    }

    impl InvalidateMockSubscriber {
        fn new() -> Rc<RefCell<InvalidateMockSubscriber>> {
            Rc::new(RefCell::new(Self { invalidate_count: 0 }))
        }
        fn reset(&mut self) {
            self.invalidate_count = 0;
        }
    }

    impl InvalidateCacheMut for InvalidateMockSubscriber {
        fn invalidate_cache(&mut self) {
            self.invalidate_count += 1;
        }
    }

    let ix1 = InvalidateMockSubscriber::new();
    x1.subscribe_to_invalidate(&(ix1.clone() as _));
    let ix2 = InvalidateMockSubscriber::new();
    x2.subscribe_to_invalidate(&(ix2.clone() as _));
    let ix3 = InvalidateMockSubscriber::new();
    x3.subscribe_to_invalidate(&(ix3.clone() as _));
    let iy1 = InvalidateMockSubscriber::new();
    y1.subscribe_to_invalidate(&(iy1.clone() as _));
    let iy2 = InvalidateMockSubscriber::new();
    y2.subscribe_to_invalidate(&(iy2.clone() as _));
    let iy3 = InvalidateMockSubscriber::new();
    y3.subscribe_to_invalidate(&(iy3.clone() as _));

    y3.compute();
    assert_eq!(ix1.borrow().invalidate_count, 0);
    assert_eq!(ix2.borrow().invalidate_count, 0);
    assert_eq!(ix3.borrow().invalidate_count, 0);
    assert_eq!(iy1.borrow().invalidate_count, 0);
    assert_eq!(iy2.borrow().invalidate_count, 0);
    assert_eq!(iy3.borrow().invalidate_count, 0);
    ix1.borrow_mut().reset();
    ix2.borrow_mut().reset();
    ix3.borrow_mut().reset();
    iy1.borrow_mut().reset();
    iy2.borrow_mut().reset();
    iy3.borrow_mut().reset();

    x1.set(1.0);
    assert_eq!(ix1.borrow().invalidate_count, 1);
    assert_eq!(ix2.borrow().invalidate_count, 0);
    assert_eq!(ix3.borrow().invalidate_count, 0);
    assert_eq!(iy1.borrow().invalidate_count, 1);
    assert_eq!(iy2.borrow().invalidate_count, 1);
    assert_eq!(iy3.borrow().invalidate_count, 1);
    ix1.borrow_mut().reset();
    ix2.borrow_mut().reset();
    ix3.borrow_mut().reset();
    iy1.borrow_mut().reset();
    iy2.borrow_mut().reset();
    iy3.borrow_mut().reset();
    y3.compute();


    x2.set(2.0);
    assert_eq!(ix1.borrow().invalidate_count, 0);
    assert_eq!(ix2.borrow().invalidate_count, 1);
    assert_eq!(ix3.borrow().invalidate_count, 0);
    assert_eq!(iy1.borrow().invalidate_count, 0);
    assert_eq!(iy2.borrow().invalidate_count, 1);
    assert_eq!(iy3.borrow().invalidate_count, 1);
    ix1.borrow_mut().reset();
    ix2.borrow_mut().reset();
    ix3.borrow_mut().reset();
    iy1.borrow_mut().reset();
    iy2.borrow_mut().reset();
    iy3.borrow_mut().reset();
    y3.compute();

    x3.set(3.0);
    assert_eq!(ix1.borrow().invalidate_count, 0);
    assert_eq!(ix2.borrow().invalidate_count, 0);
    assert_eq!(ix3.borrow().invalidate_count, 1);
    assert_eq!(iy1.borrow().invalidate_count, 0);
    assert_eq!(iy2.borrow().invalidate_count, 0);
    assert_eq!(iy3.borrow().invalidate_count, 1);
    ix1.borrow_mut().reset();
    ix2.borrow_mut().reset();
    ix3.borrow_mut().reset();
    iy1.borrow_mut().reset();
    iy2.borrow_mut().reset();
    iy3.borrow_mut().reset();
    y3.compute();

    x1.set(11.0);
    x2.set(12.0);
    x3.set(13.0);
    assert_eq!(ix1.borrow().invalidate_count, 1);
    assert_eq!(ix2.borrow().invalidate_count, 1);
    assert_eq!(ix3.borrow().invalidate_count, 1);
    assert_eq!(iy1.borrow().invalidate_count, 1);
    assert_eq!(iy2.borrow().invalidate_count, 1);
    assert_eq!(iy3.borrow().invalidate_count, 1);
}

