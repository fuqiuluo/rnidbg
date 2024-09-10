use std::collections::HashMap;
use std::rc::Rc;
use crate::android::dvm::class::DvmClass;
use crate::android::jni::{generate_class_id};

pub struct ClassResolver {
    class_map: HashMap<i64, Rc<DvmClass>>,
}

impl ClassResolver {
    /// This method will build a class resolver,
    /// the input data will be de-ordered,
    ///
    /// # Will be automatically added
    /// - `java.lang.NoClassDefFoundError`
    /// - `java.lang.NoSuchFieldError`
    /// - `java.lang.NoSuchMethodError`,
    /// - `boolean[]`
    /// - `byte[]`
    /// - `char[]`
    /// - `short[]`
    /// - `int[]`
    /// - `long[]`
    /// - `float[]`
    /// - `double[]`
    /// - `java.lang.Boolean`
    /// - `java.lang.Byte`
    /// - `java.lang.Character`
    /// - `java.lang.Short`
    /// - `java.lang.Integer`
    /// - `java.lang.Long`
    /// - `java.lang.Float`
    /// - `java.lang.Double`
    /// - `java.lang.String`
    /// - `java.lang.Object`
    ///
    /// The `class_id` will be generated from `JNI_ID_BASE` and increased by 0x10000 each time.
    ///
    /// # Example
    /// ```no_run
    /// use emulator::android::dvm::class_resolver::ClassResolver;
    ///
    /// let class_resolver = ClassResolver::new(vec![
    ///    "java/lang/Object",
    ///    "java/lang/String",
    /// ]);
    pub fn new(
        mut class_list: Vec<&str>
    ) -> Self {
        class_list.push("java/lang/NoSuchFieldError");
        class_list.push("java/lang/NoSuchMethodError");
        class_list.push("java/lang/NoClassDefFoundError");
        class_list.push("boolean[]");
        class_list.push("byte[]");
        class_list.push("char[]");
        class_list.push("short[]");
        class_list.push("int[]");
        class_list.push("long[]");
        class_list.push("float[]");
        class_list.push("double[]");
        class_list.push("java/lang/Boolean");
        class_list.push("java/lang/Byte");
        class_list.push("java/lang/Character");
        class_list.push("java/lang/Short");
        class_list.push("java/lang/Integer");
        class_list.push("java/lang/Long");
        class_list.push("java/lang/Float");
        class_list.push("java/lang/Double");
        class_list.push("java/lang/String");
        class_list.push("java/lang/Object");

        class_list.sort();
        class_list.dedup();

        let mut seq = 0;
        let mut map = HashMap::new();
        for dvm_class in class_list {
            let id = generate_class_id(seq);
            map.insert(id, Rc::new(DvmClass::new_class(id, dvm_class)));
            seq += 1;
        }
        ClassResolver {
            class_map: map,
        }
    }

    /*/// I don't recommend this method at all!
    fn new_with_class(
        mut class_list: Vec<(i64, Rc<DvmClass>)>
    ) -> Self {
        class_list.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        class_list.dedup_by(|a, b| a.1.name == b.1.name);
        let mut seq = 0;
        let mut map = HashMap::new();
        for dvm_class in class_list {
            map.insert(generate_class_id(seq), dvm_class.1);
            seq += 1;
        }
        ClassResolver {
            class_map: map,
        }
    }*/

    pub fn find_class_by_name(&self, name: &str) -> Option<(i64, Rc<DvmClass>)> {
        for (id, class) in self.class_map.iter() {
            if class.name == name {
                return Some((*id, Rc::clone(class)));
            }
        }
        None
    }

    pub fn find_class_by_id(&self, id: &i64) -> Option<(i64, Rc<DvmClass>)> {
        self.class_map.get(id).map(|cz| (*id, cz.clone()))
    }
}