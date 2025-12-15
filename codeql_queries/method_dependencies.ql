/**
 * @name Method Dependencies
 * @description Extract all method call dependencies for Firebase Auth and Firestore
 * @kind table
 * @id cpp/firebase/method-dependencies
 */

import cpp

from MemberFunction caller, Function callee, FunctionCall call
where 
  call.getEnclosingFunction() = caller and
  call.getTarget() = callee and
  (
    caller.getDeclaringType().getQualifiedName().matches("firebase::auth::%") or
    caller.getDeclaringType().getQualifiedName().matches("firebase::firestore::%")
  ) and
  // Only track calls to firebase namespace or standard library
  (
    callee.getQualifiedName().matches("firebase::%") or
    callee.getQualifiedName().matches("std::%") or
    callee.getQualifiedName().matches("firebase::auth::%") or
    callee.getQualifiedName().matches("firebase::firestore::%")
  )
select 
  caller.getDeclaringType().getQualifiedName() as caller_class,
  caller.getName() as caller_method,
  caller.getQualifiedName() as caller_qualified,
  callee.getQualifiedName() as callee_function,
  call.getLocation().getStartLine() as line_number
