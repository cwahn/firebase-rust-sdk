/**
 * @name Field Dependencies
 * @description Extract field types from classes
 * @kind table
 * @id cpp/firebase/field-dependencies
 */

import cpp

from Class c, Field f, Type t
where 
  f.getDeclaringType() = c and
  t = f.getType().getUnspecifiedType() and
  (
    c.getQualifiedName().matches("firebase::auth::%") or
    c.getQualifiedName().matches("firebase::firestore::%")
  ) and
  // Filter out primitives
  not t instanceof VoidType and
  not t instanceof IntegralType and
  not t instanceof FloatingPointType and
  not t instanceof BoolType
select 
  c.getQualifiedName() as class_name,
  f.getName() as field_name,
  "field" as dependency_kind,
  t.toString() as type_name,
  t.toString() as type_qualified,
  c.getFile().getRelativePath() as file_path,
  f.getLocation().getStartLine() as line_number
