use rand::Rng;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DataType {
    Bool = 1,
    SInt = 2,
    UInt = 4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TypeConstraints(u32);

impl TypeConstraints {
    pub const BOOL: TypeConstraints = TypeConstraints(1);
    pub const SINT: TypeConstraints = TypeConstraints(2);
    pub const UINT: TypeConstraints = TypeConstraints(4);

    pub const INT: TypeConstraints = TypeConstraints::SINT.union(TypeConstraints::UINT);
    pub const UNCONSTRAINED: TypeConstraints = TypeConstraints::BOOL.union(TypeConstraints::INT);

    pub fn any_of(i: impl IntoIterator<Item = DataType>) -> Self {
        let mut v = 0;
        for t in i {
            v |= t as u32;
        }
        TypeConstraints(v)
    }

    pub const fn union(self, other: TypeConstraints) -> TypeConstraints {
        TypeConstraints(self.0 | other.0)
    }

    pub const fn intersection(self, other: TypeConstraints) -> Option<TypeConstraints> {
        let intersection = self.0 & other.0;
        if intersection == 0 {
            None
        } else {
            Some(TypeConstraints(intersection))
        }
    }

    pub fn select(self) -> DataType {
        debug_assert_ne!(self.0, 0);

        let n = rand::thread_rng().gen_range(0..self.0.count_ones());
        let mut j = 0;

        for i in 0..32 {
            if self.0 & (1 << i) != 0 {
                if j == n {
                    return match 1 << i {
                        1 => DataType::Bool,
                        2 => DataType::SInt,
                        4 => DataType::UInt,
                        _ => unreachable!(),
                    };
                } else {
                    j += 1;
                }
            }
        }

        // This should be unreachable as long as the constraints are never empty
        // i.e. self.0 != 0
        unreachable!()
    }
}
