import { Action, ActionData, Moderation } from "./valeriyya.moderation";
import { ValeriyyaEmbed } from "../valeriyya.embed";
import { reply } from "../valeriyya.util";

type BanData = Omit<ActionData, "duration">;

export class Ban extends Moderation {
  public constructor(data: BanData) {
    super(Action.BAN, data);
  }

  public permissions() {
    if (!this.int.memberPermissions?.has("BAN_MEMBERS", true)) {
      const embed = new ValeriyyaEmbed(undefined, "error")
        .setAuthor({ name: `${this.int.user.tag} (${this.int.user.id})`, url: this.int.user.displayAvatarURL({ dynamic: true }) })
        .setDescription("You are missing the `BAN_MEMBERS` permission");
      reply(this.int, { embeds: [embed] });
      return false;
    }
    return true;
  }

  public async execute(): Promise<void> {
    const db = await this.client.db(this.int.guild!);
    const history_number = (await db.getUserHistory(this.target.id))!.ban + 1;
    const cases_number = db.cases_number + 1;

    try {
      await this.int.guild?.members.ban(this.target, { reason: `Case ${cases_number}` });
    } catch (e: any) {
      reply(this.int, { content: `There was an error banning this member: ${e}`, ephemeral: true });
      this.client.logger.error(`There was an error with the moderation-BAN method: ${e}`);
    }

    db.cases_number = cases_number;
    db.history.find((m) => m.id === this.target.id)!.ban = history_number;
    await db.save();
  }
}
