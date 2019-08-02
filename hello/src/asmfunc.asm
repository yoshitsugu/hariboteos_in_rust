[BITS 32]
	GLOBAL	_api_initmalloc
    GLOBAL	_api_malloc
	GLOBAL	_api_free
	GLOBAL	_api_linewin

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
_api_linewin:		; void api_linewin(int win, int x0, int y0, int x1, int y1, int col);
		PUSH	EDI
		PUSH	ESI
		PUSH	EBP
		PUSH	EBX
		MOV		EDX,13
		MOV		EBX,[ESP+20]	; win
		MOV		EAX,[ESP+24]	; x0
		MOV		ECX,[ESP+28]	; y0
		MOV		ESI,[ESP+32]	; x1
		MOV		EDI,[ESP+36]	; y1
		MOV		EBP,[ESP+40]	; col
		INT		0x40
		POP		EBX
		POP		EBP
		POP		ESI
		POP		EDI
		RET
