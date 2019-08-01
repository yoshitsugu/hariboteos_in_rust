[BITS 32]
	GLOBAL	_api_initmalloc
    GLOBAL	_api_malloc
	GLOBAL	_api_free

[SECTION .text]

_api_initmalloc:	; void api_initmalloc(void);
		PUSH	EBX
		MOV		EDX,8
		MOV		EBX,[CS:0x0020]		; malloc領域の番地
		MOV		EAX,EBX
		ADD		EAX,32*1024			; 32KBを足す
		MOV		ECX,[CS:0x0000]		; データセグメントの大きさ
		SUB		ECX,EAX
		INT		0x40
		POP		EBX
		RET
_api_malloc:		; char *api_malloc(int size);
		PUSH	EBX
		MOV		EDX,9
		MOV		EBX,[CS:0x0020]
		MOV		ECX,[ESP+8]			; size
		INT		0x40
		POP		EBX
		RET

_api_free:			; void api_free(char *addr, int size);
		PUSH	EBX
		MOV		EDX,10
		MOV		EBX,[CS:0x0020]
		MOV		EAX,[ESP+ 8]		; addr
		MOV		ECX,[ESP+12]		; size
		INT		0x40
		POP		EBX
		RET
