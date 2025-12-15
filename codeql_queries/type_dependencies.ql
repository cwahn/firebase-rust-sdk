/**
 * @name Type Dependencies
 * @description Extract type usage in methods (parameters, returns, fields)
 * @kind table
 * @id cpp/firebase/type-dependencies
 */

import cpp

// Method parameter types
from MemberFunction m, Parameter p, Type t
where 
  p.getFunction() = m and
  t = p.getType().getUnspecifiedType() and
  (
    m.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    m.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  ) and
  // Filter out primitive types and void
  not t instanceof VoidType and
  not t instanceof IntegralType and
  not t instanceof FloatingPointType and
  not t instanceof BoolType
select 
  m.getDeclaringType().getQualifiedName() as class_name,
  m.getName() as method_name,
  m.getQualifiedName() as method_qualified,
  "parameter" as dependency_kind,
  p.getName() as param_name,
  t.toString() as type_name,
  t.toString() as type_qualified,
  m.getFile().getRelativePath() as file_path,
  m.getLocation().getStartLine() as line_number
