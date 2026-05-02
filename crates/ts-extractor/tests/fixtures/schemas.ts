export interface UserType {
    id: string;
    name: string;
    email: string;
    role: UserRole;
}

export type UserRole = 'admin' | 'user' | 'guest';

export enum Permission {
    Read = 'read',
    Write = 'write',
    Admin = 'admin',
}

export class ValidationError extends Error {
    constructor(public field: string, message: string) {
        super(message);
    }
}
