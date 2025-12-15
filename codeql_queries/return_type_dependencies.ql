/**
 * @name Return Type Dependencies
 * @description Extract return types from all public methods
 * @kind table
 * @id cpp/firebase/return-type-dependencies
 */

import cpp

from MemberFunction m, Type t
where 
  t = m.getType().getUnspecifiedType() and
  m.isPublic() and
  (
    m.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    m.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  ) and
  // Filter out void and primitives
  not t instanceof VoidType and
  not t instanceof IntegralType and
  not t instanceof FloatingPointType and
  not t instanceof BoolType
select 
  m.getDeclaringType().getQualifiedName() as class_name,
  m.getName() as method_name,
  m.getQualifiedName() as method_qualified,
  "return" as dependency_kind,
  t.toString() as type_name,
  t.toString() as type_qualified,
  m.getFile().getRelativePath() as file_path,
  m.getLocation().getStartLine() as line_number
