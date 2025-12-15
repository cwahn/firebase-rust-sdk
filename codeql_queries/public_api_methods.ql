/**
 * @name Public API Methods
 * @description Extract all public methods from Auth and Firestore classes
 * @kind table
 * @id cpp/firebase/public-api-methods
 */

import cpp

from Class c, MemberFunction m
where 
  m.getDeclaringType() = c and
  m.isPublic() and
  (
    c.getQualifiedName().matches("firebase::auth::%") or
    c.getQualifiedName().matches("firebase::firestore::%")
  ) and
  // Only include methods from public headers
  c.getFile().getRelativePath().matches("%/include/%")
select 
  c.getQualifiedName() as class_name,
  m.getName() as method_name,
  m.getQualifiedName() as method_qualified,
  m.getType().toString() as return_type,
  m.getNumberOfParameters() as param_count,
  m.getFile().getRelativePath() as file_path
