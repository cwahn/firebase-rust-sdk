/**
 * @name Include Dependencies
 * @description Extract header file dependencies
 * @kind table
 * @id cpp/firebase/include-dependencies
 */

import cpp

from Include inc
where 
  (
    inc.getFile().getRelativePath().matches("%firebase/auth%") or
    inc.getFile().getRelativePath().matches("%firebase/firestore%")
  ) and
  inc.getFile().getRelativePath().matches("%/include/%")
select 
  inc.getFile().getRelativePath() as including_file,
  inc.getIncludedFile().getRelativePath() as included_file,
  inc.getLocation().getStartLine() as line_number
