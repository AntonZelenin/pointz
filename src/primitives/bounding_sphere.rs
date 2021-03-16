use crate::model::SimpleVertex;

const VERTICES: [SimpleVertex; 12] = [
    SimpleVertex {
        position: [0.0, -1.0, 1.0],
    },
    SimpleVertex {
        position: [0.0, 1.0, 1.0],
    },
    SimpleVertex {
        position: [0.0, 1.0, -1.0],
    },
    SimpleVertex {
        position: [0.0, -1.0, -1.0],
    },

    SimpleVertex {
        position: [-1.0, -1.0, 0.0],
    },
    SimpleVertex {
        position: [-1.0, 1.0, 0.0],
    },
    SimpleVertex {
        position: [1.0, 1.0, 0.0],
    },
    SimpleVertex {
        position: [1.0, -1.0, 0.0],
    },


    SimpleVertex {
        position: [-1.0, 0.0, 1.0],
    },
    SimpleVertex {
        position: [1.0, 0.0, 1.0],
    },
    SimpleVertex {
        position: [1.0, 0.0, -1.0],
    },
    SimpleVertex {
        position: [-1.0, 0.0, -1.0],
    },
];
const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 1,

    4, 5, 6,
    6, 7, 4,

    8, 9, 10,
    10, 11, 8,
];
