#[macro_use]
mod compgraph;

struct Kek<T> {
    a: T
}

fn test() {
}

#[cfg(test)]
mod tests {
    use super::compgraph::*;

    define_node!(AddNode computes add(a, b) { a + b });
    
    #[test]
    fn it_works() {
        let graph = add(2.0, 4.0);
        assert_eq!(graph.compute(), 6.0);
    }
}

