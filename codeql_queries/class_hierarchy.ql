/**
 * @name Class Hierarchy
 * @description Extract all class inheritance relationships in Firebase Auth and Firestore
 * @kind table
 * @id cpp/firebase/class-hierarchy
 */

import cpp

from Class c, Class base
where 
  c.getABaseClass() = base and
  (
    c.getQualifiedName().matches("firebase::auth::%") or
    c.getQualifiedName().matches("firebase::firestore::%") or
    base.getQualifiedName().matches("firebase::auth::%") or
    base.getQualifiedName().matches("firebase::firestore::%")
  )
select 
  c.getQualifiedName() as derived_class,
  base.getQualifiedName() as base_class,
  c.getFile().getRelativePath() as file_path
