if exists('b:current_syntax')
  finish
endif

syn keyword ralKeywords instruments score init perf print println output local skipwhite
syn keyword ralTypes Int Float Audio String skipwhite

syn keyword ralTodo TODO FIXME NOTES NOTE XXX contained
syn match ralComment "//.*$" contains=ralTodo

syn match ralNumber "\v<\d+>"
syn match ralNumber "\v0[xX]\x+"
syn match ralNumber "\v0[bB]\d+"
syn match ralNumber "\v<\d+\.\d+>"
syn region ralString start='"' end='"'

hi def link ralTodo Todo
hi def link ralComment Comment
hi def link ralString String
hi def link ralNumber Number
hi def link ralKeywords Keyword
hi def link ralTypes Type

let b:current_syntax = "ral"
