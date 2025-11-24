declare module 'zmodem.js' {
  export class Sentry {
    constructor(options: any);
    consume(data: any): void;
  }
  export namespace Browser {
    function send_files(session: any, files: any[], options?: any): Promise<void>;
  }
}
